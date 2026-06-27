use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use futures_core::Stream;
use knx_core::{
    Apci, CemiFrame, ConnectionHeader, GroupAddress, HostProtocol, Hpai, IndividualAddress,
    KnxNetIpHeader, ServiceType,
};
use knx_dpt::DptValue;
use tokio::net::UdpSocket;
use tokio::sync::broadcast;
use tokio::time;
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt;

use crate::telegram::GroupEvent;
use crate::{ConnectionEvent, HeartbeatOptions, KnxIpError, Result, TunnelOptions};

const RECEIVE_BUFFER_LEN: usize = 1500;

#[derive(Debug)]
pub struct TunnelClient {
    socket: Arc<UdpSocket>,
    target: SocketAddr,
    channel_id: u8,
    sequence_counter: u8,
    source: IndividualAddress,
    ack_tx: broadcast::Sender<u8>,
    event_tx: broadcast::Sender<GroupEvent>,
    heartbeat_tx: broadcast::Sender<u8>,
    disconnect_tx: broadcast::Sender<u8>,
    ack_timeout: Duration,
    heartbeat: Option<HeartbeatOptions>,
    control_endpoint: SocketAddr,
    lifecycle_events: Vec<ConnectionEvent>,
}

impl TunnelClient {
    pub async fn connect(target: SocketAddr) -> Result<Self> {
        Self::connect_with_options(TunnelOptions::new(target)).await
    }

    pub async fn connect_with_options(options: TunnelOptions) -> Result<Self> {
        let Some(policy) = options.reconnect_policy else {
            return Self::connect_once(options, Vec::new()).await;
        };

        let mut attempt = 1;
        let mut delay = policy.initial_delay;
        let mut lifecycle_events = Vec::new();

        loop {
            match Self::connect_once(options, lifecycle_events.clone()).await {
                Ok(mut client) => {
                    if attempt > 1 {
                        client.lifecycle_events.push(ConnectionEvent::Reconnected {
                            attempt,
                            channel_id: client.channel_id,
                        });
                    }
                    return Ok(client);
                }
                Err(_) if attempt >= policy.max_attempts => {
                    return Err(KnxIpError::ReconnectAttemptsExhausted { attempts: attempt });
                }
                Err(_) => {
                    let next_attempt = attempt + 1;
                    lifecycle_events.push(ConnectionEvent::Reconnecting {
                        attempt: next_attempt,
                        delay,
                    });
                    time::sleep(delay).await;
                    delay = policy.next_delay(delay);
                    attempt = next_attempt;
                }
            }
        }
    }

    async fn connect_once(
        options: TunnelOptions,
        lifecycle_events: Vec<ConnectionEvent>,
    ) -> Result<Self> {
        ensure_ipv4(options.target)?;
        ensure_ipv4(options.bind)?;
        if let Some(endpoint) = options.control_endpoint {
            ensure_ipv4(endpoint)?;
        }
        if let Some(endpoint) = options.data_endpoint {
            ensure_ipv4(endpoint)?;
        }

        let socket = UdpSocket::bind(options.bind).await?;
        let local_addr = socket.local_addr()?;
        let control_endpoint = options.control_endpoint.unwrap_or(local_addr);
        let data_endpoint = options.data_endpoint.unwrap_or(local_addr);
        let request = encode_connect_request(control_endpoint, data_endpoint)?;

        socket.send_to(&request, options.target).await?;

        let mut buffer = [0_u8; RECEIVE_BUFFER_LEN];
        let (len, _) = time::timeout(options.ack_timeout, socket.recv_from(&mut buffer))
            .await
            .map_err(|_| KnxIpError::Timeout)??;
        let channel_id = decode_connect_response(&buffer[..len])?;

        let socket = Arc::new(socket);
        let (ack_tx, _) = broadcast::channel(16);
        let (event_tx, _) = broadcast::channel(64);
        let (heartbeat_tx, _) = broadcast::channel(16);
        let (disconnect_tx, _) = broadcast::channel(16);

        spawn_receive_loop(
            socket.clone(),
            options.target,
            channel_id,
            ack_tx.clone(),
            event_tx.clone(),
            heartbeat_tx.clone(),
            disconnect_tx.clone(),
        );

        Ok(Self {
            socket,
            target: options.target,
            channel_id,
            sequence_counter: 0,
            source: IndividualAddress::from_raw(0),
            ack_tx,
            event_tx,
            heartbeat_tx,
            disconnect_tx,
            ack_timeout: options.ack_timeout,
            heartbeat: options.heartbeat,
            control_endpoint,
            lifecycle_events,
        })
    }

    pub async fn connect_with_ack_timeout(
        target: SocketAddr,
        ack_timeout: Duration,
    ) -> Result<Self> {
        let mut options = TunnelOptions::new(target);
        options.ack_timeout = ack_timeout;
        Self::connect_with_options(options).await
    }

    pub async fn group_write(&mut self, group: GroupAddress, value: DptValue) -> Result<()> {
        let payload = encode_value(value)?;
        let frame = CemiFrame::group_value_write(self.source, group, &payload)?;

        self.send_tunnelling_frame(frame).await
    }

    /// Send a GroupValueRead request without waiting for a response
    /// (the answer, if any, arrives as a bus indication observable via
    /// [`TunnelClient::monitor`]). Completes after the tunnelling ack.
    pub async fn group_read_request(&mut self, group: GroupAddress) -> Result<()> {
        let frame = CemiFrame::group_value_read(self.source, group)?;

        self.send_tunnelling_frame(frame).await
    }

    /// Send a GroupValueResponse telegram (answering a group read).
    pub async fn group_response(&mut self, group: GroupAddress, value: DptValue) -> Result<()> {
        let payload = encode_value(value)?;
        let frame = CemiFrame::group_value_response(self.source, group, &payload)?;

        self.send_tunnelling_frame(frame).await
    }

    pub async fn group_read(
        &mut self,
        group: GroupAddress,
        dpt: &str,
        timeout: Duration,
    ) -> Result<DptValue> {
        let mut events = self.event_tx.subscribe();
        let frame = CemiFrame::group_value_read(self.source, group)?;
        self.send_tunnelling_frame(frame).await?;

        let deadline = time::Instant::now() + timeout;
        loop {
            let Some(remaining) = deadline.checked_duration_since(time::Instant::now()) else {
                return Err(KnxIpError::Timeout);
            };

            let event = match time::timeout(remaining, events.recv()).await {
                Ok(Ok(event)) => event,
                Ok(Err(broadcast::error::RecvError::Lagged(_))) => continue,
                Ok(Err(broadcast::error::RecvError::Closed)) => {
                    return Err(KnxIpError::ReceiveLoopStopped);
                }
                Err(_) => return Err(KnxIpError::Timeout),
            };

            if event.destination == group && event.apci == Apci::GroupValueResponse {
                return Ok(knx_dpt::decode(dpt, &event.payload)?);
            }
        }
    }

    pub fn monitor(&mut self) -> impl Stream<Item = Result<GroupEvent>> + Send + 'static {
        BroadcastStream::new(self.event_tx.subscribe()).map(|event| match event {
            Ok(event) => Ok(event),
            Err(_) => Err(KnxIpError::MonitorLagged),
        })
    }

    pub fn lifecycle_events(&self) -> &[ConnectionEvent] {
        &self.lifecycle_events
    }

    pub async fn heartbeat(&self) -> Result<()> {
        let heartbeat = self
            .heartbeat
            .ok_or(KnxIpError::InvalidResponse("heartbeat is not configured"))?;

        let mut missed = 0;
        loop {
            time::sleep(heartbeat.interval).await;
            match self.heartbeat_once(heartbeat.timeout).await {
                Ok(()) => return Ok(()),
                Err(KnxIpError::HeartbeatTimeout) if missed + 1 < heartbeat.max_missed => {
                    missed += 1;
                }
                Err(error) => return Err(error),
            }
        }
    }

    async fn heartbeat_once(&self, timeout: Duration) -> Result<()> {
        let mut responses = self.heartbeat_tx.subscribe();
        let packet = encode_connection_state_request(self.channel_id, self.control_endpoint)?;
        self.socket.send_to(&packet, self.target).await?;

        loop {
            match time::timeout(timeout, responses.recv()).await {
                Ok(Ok(0x00)) => return Ok(()),
                Ok(Ok(status)) => return Err(KnxIpError::GatewayStatus { status }),
                Ok(Err(broadcast::error::RecvError::Lagged(_))) => continue,
                Ok(Err(broadcast::error::RecvError::Closed)) => {
                    return Err(KnxIpError::ReceiveLoopStopped);
                }
                Err(_) => return Err(KnxIpError::HeartbeatTimeout),
            }
        }
    }

    /// Send a KNXnet/IP DISCONNECT_REQUEST and wait for the matching
    /// DISCONNECT_RESPONSE, bounded by the ACK timeout.
    ///
    /// This is the orderly counterpart to the CONNECT_REQUEST handshake:
    /// it frees the gateway's tunnel slot immediately instead of relying
    /// on the gateway's connection-state timeout. It is timeout-safe (it
    /// never blocks longer than the ACK timeout) — on a silent gateway it
    /// returns [`KnxIpError::DisconnectTimeout`] rather than hanging. The
    /// caller (binding `close()`) treats it as best-effort: the request
    /// is a real frame on the wire, and local teardown proceeds whether
    /// or not the response arrives.
    pub async fn disconnect(&self) -> Result<()> {
        let mut responses = self.disconnect_tx.subscribe();
        let packet = encode_disconnect_request(self.channel_id, self.control_endpoint)?;
        self.socket.send_to(&packet, self.target).await?;

        loop {
            match time::timeout(self.ack_timeout, responses.recv()).await {
                Ok(Ok(0x00)) => return Ok(()),
                Ok(Ok(status)) => return Err(KnxIpError::GatewayStatus { status }),
                Ok(Err(broadcast::error::RecvError::Lagged(_))) => continue,
                Ok(Err(broadcast::error::RecvError::Closed)) => {
                    return Err(KnxIpError::ReceiveLoopStopped);
                }
                Err(_) => return Err(KnxIpError::DisconnectTimeout),
            }
        }
    }

    async fn send_tunnelling_frame(&mut self, frame: CemiFrame) -> Result<()> {
        let sequence = self.sequence_counter;
        self.sequence_counter = self.sequence_counter.wrapping_add(1);

        let packet = encode_tunnelling_request(self.channel_id, sequence, &frame)?;
        let mut acks = self.ack_tx.subscribe();

        self.socket.send_to(&packet, self.target).await?;

        loop {
            match time::timeout(self.ack_timeout, acks.recv()).await {
                Ok(Ok(ack_sequence)) if ack_sequence == sequence => return Ok(()),
                Ok(Ok(_)) => continue,
                Ok(Err(broadcast::error::RecvError::Lagged(_))) => continue,
                Ok(Err(broadcast::error::RecvError::Closed)) => {
                    return Err(KnxIpError::ReceiveLoopStopped);
                }
                Err(_) => return Err(KnxIpError::AckTimeout),
            }
        }
    }
}

fn spawn_receive_loop(
    socket: Arc<UdpSocket>,
    target: SocketAddr,
    channel_id: u8,
    ack_tx: broadcast::Sender<u8>,
    event_tx: broadcast::Sender<GroupEvent>,
    heartbeat_tx: broadcast::Sender<u8>,
    disconnect_tx: broadcast::Sender<u8>,
) {
    tokio::spawn(async move {
        let mut buffer = [0_u8; RECEIVE_BUFFER_LEN];

        loop {
            let Ok((len, peer)) = socket.recv_from(&mut buffer).await else {
                break;
            };
            if peer != target {
                continue;
            }

            let Ok((header, payload)) = KnxNetIpHeader::decode(&buffer[..len]) else {
                continue;
            };

            match header.service_type() {
                ServiceType::TunnellingAck => {
                    if let Ok((connection, _)) = ConnectionHeader::decode(payload) {
                        if connection.channel_id() == channel_id && connection.status() == 0 {
                            let _ = ack_tx.send(connection.sequence_counter());
                        }
                    }
                }
                ServiceType::TunnellingRequest => {
                    handle_tunnelling_request(&socket, target, channel_id, payload, &event_tx)
                        .await;
                }
                ServiceType::ConnectionStateResponse => {
                    handle_connection_state_response(channel_id, payload, &heartbeat_tx);
                }
                ServiceType::DisconnectResponse => {
                    handle_disconnect_response(channel_id, payload, &disconnect_tx);
                }
                _ => {}
            }
        }
    });
}

fn handle_connection_state_response(
    channel_id: u8,
    payload: &[u8],
    heartbeat_tx: &broadcast::Sender<u8>,
) {
    if payload.len() < 2 || payload[0] != channel_id {
        return;
    }

    let _ = heartbeat_tx.send(payload[1]);
}

fn handle_disconnect_response(
    channel_id: u8,
    payload: &[u8],
    disconnect_tx: &broadcast::Sender<u8>,
) {
    if payload.len() < 2 || payload[0] != channel_id {
        return;
    }

    let _ = disconnect_tx.send(payload[1]);
}

async fn handle_tunnelling_request(
    socket: &UdpSocket,
    target: SocketAddr,
    channel_id: u8,
    payload: &[u8],
    event_tx: &broadcast::Sender<GroupEvent>,
) {
    let Ok((connection, payload)) = ConnectionHeader::decode(payload) else {
        return;
    };
    if connection.channel_id() != channel_id {
        return;
    }

    let _ = send_tunnelling_ack(socket, target, channel_id, connection.sequence_counter()).await;

    let Ok((frame, remaining)) = CemiFrame::decode(payload) else {
        return;
    };
    if !remaining.is_empty() {
        return;
    }

    let _ = event_tx.send(GroupEvent::from_cemi(&frame));
}

fn encode_connect_request(
    control_endpoint: SocketAddr,
    data_endpoint: SocketAddr,
) -> Result<Vec<u8>> {
    let mut payload = Vec::new();
    hpai_for(control_endpoint)?.encode(&mut payload)?;
    hpai_for(data_endpoint)?.encode(&mut payload)?;
    payload.extend_from_slice(&[0x04, 0x04, 0x02, 0x00]);

    encode_knxnetip(ServiceType::ConnectRequest, &payload)
}

fn decode_connect_response(input: &[u8]) -> Result<u8> {
    let (header, payload) = KnxNetIpHeader::decode(input)?;
    if header.service_type() != ServiceType::ConnectResponse {
        return Err(KnxIpError::InvalidResponse("expected connect response"));
    }
    if payload.len() < 2 {
        return Err(KnxIpError::InvalidResponse("truncated connect response"));
    }
    if payload[1] != 0 {
        return Err(KnxIpError::InvalidResponse("connect response rejected"));
    }

    Ok(payload[0])
}

fn encode_tunnelling_request(channel_id: u8, sequence: u8, frame: &CemiFrame) -> Result<Vec<u8>> {
    let mut payload = Vec::new();
    ConnectionHeader::new(channel_id, sequence, 0).encode(&mut payload)?;
    frame.encode(&mut payload)?;

    encode_knxnetip(ServiceType::TunnellingRequest, &payload)
}

fn encode_connection_state_request(
    channel_id: u8,
    control_endpoint: SocketAddr,
) -> Result<Vec<u8>> {
    let mut payload = vec![channel_id, 0x00];
    hpai_for(control_endpoint)?.encode(&mut payload)?;

    encode_knxnetip(ServiceType::ConnectionStateRequest, &payload)
}

fn encode_disconnect_request(channel_id: u8, control_endpoint: SocketAddr) -> Result<Vec<u8>> {
    let mut payload = vec![channel_id, 0x00];
    hpai_for(control_endpoint)?.encode(&mut payload)?;

    encode_knxnetip(ServiceType::DisconnectRequest, &payload)
}

async fn send_tunnelling_ack(
    socket: &UdpSocket,
    target: SocketAddr,
    channel_id: u8,
    sequence: u8,
) -> Result<()> {
    let mut payload = Vec::new();
    ConnectionHeader::new(channel_id, sequence, 0).encode(&mut payload)?;
    let packet = encode_knxnetip(ServiceType::TunnellingAck, &payload)?;

    socket.send_to(&packet, target).await?;
    Ok(())
}

fn encode_knxnetip(service_type: ServiceType, payload: &[u8]) -> Result<Vec<u8>> {
    let total_length = u16::try_from(6 + payload.len())
        .map_err(|_| KnxIpError::InvalidResponse("KNXnet/IP frame too long"))?;
    let mut frame = Vec::new();
    KnxNetIpHeader::new(service_type, total_length)?.encode(&mut frame)?;
    frame.extend_from_slice(payload);

    Ok(frame)
}

pub(crate) fn encode_value(value: DptValue) -> Result<Vec<u8>> {
    let dpt = match &value {
        DptValue::Bool(_) => "1.001",
        DptValue::U8(_) => "5.010",
        DptValue::Scaling(_) => "5.001",
        DptValue::Temperature(_) => "9.001",
        DptValue::ControlBool { .. } => "2.001",
        DptValue::StepControl { .. } => "3.007",
        DptValue::I8(_) => "6.010",
        DptValue::U16(_) => "7.001",
        DptValue::I16(_) => "8.001",
        DptValue::Time { .. } => "10.001",
        DptValue::Date { .. } => "11.001",
        DptValue::DateTime { .. } => "19.001",
        DptValue::U32(_) => "12.001",
        DptValue::I32(_) => "13.001",
        DptValue::F32(_) => "14.000",
        DptValue::Text14(_) => "16.000",
        DptValue::SceneNumber(_) => "17.001",
        DptValue::SceneControl { .. } => "18.001",
        DptValue::HvacMode(_) => "20.102",
        DptValue::HvacControllerMode(_) => "20.105",
        // Float16 (2-octet floats 9.002/9.003/9.004..9.008) and Angle (5.003)
        // are decode-only: a single variant cannot infer one DPT sub-type, so
        // they are not writable through variant-keyed inference.
        DptValue::Float16(_)
        | DptValue::Angle(_)
        | DptValue::Rgb { .. }
        | DptValue::Rgbw { .. }
        | DptValue::EnergyI32(_)
        | DptValue::EnergyU32(_)
        | DptValue::I64(_)
        | DptValue::Char(_)
        | DptValue::Bitset8(_)
        | DptValue::Bitset16(_) => {
            return Err(
                knx_dpt::DptError::UnsupportedDpt("unconfirmed DPT category".to_owned()).into(),
            );
        }
    };

    Ok(knx_dpt::encode(dpt, value)?)
}

fn hpai_for(endpoint: SocketAddr) -> Result<Hpai> {
    let SocketAddr::V4(endpoint) = endpoint else {
        return Err(KnxIpError::UnsupportedIpv6);
    };

    Ok(Hpai::new(
        HostProtocol::Ipv4Udp,
        endpoint.ip().octets(),
        endpoint.port(),
    ))
}

fn ensure_ipv4(endpoint: SocketAddr) -> Result<()> {
    match endpoint {
        SocketAddr::V4(_) => Ok(()),
        SocketAddr::V6(_) => Err(KnxIpError::UnsupportedIpv6),
    }
}

#[cfg(test)]
mod tests {
    use super::encode_value;
    use crate::KnxIpError;
    use knx_dpt::{DptError, DptValue};

    #[test]
    fn encode_value_refuses_decode_only_variants() {
        // Values from decode-only or inference-ambiguous DPTs must be rejected by
        // variant-keyed write inference so they cannot be silently written to the
        // wrong main. Rgb is encodable at the pure codec level
        // (knx_dpt::encode("232.600", Rgb) -> 3 bytes), but bus writes still
        // refuse Rgb/Rgbw through this inferred path.
        // EnergyI32/EnergyU32 stay refused on the actuation path too: a single
        // Energy variant cannot infer which 13.xxx sub (13.010 Wh / 13.013 kWh /
        // 13.014 VAh / 13.015 VARh) to emit, so it must never be inference-written.
        // This holds even though the four energy subs (13.010/13.013/13.014/13.015)
        // decode to EnergyI32 and can be encoded by explicit DPT id. The offline
        // codec is DPT-id-keyed; this live path is variant-keyed and stays refused.
        for value in [
            DptValue::I64(9_223_372_036_854_775_807),
            DptValue::I64(-1),
            DptValue::Float16(5.0),
            // Angle (5.003) is decode-only and must never silently drift to a
            // writable inferred arm.
            DptValue::Angle(5.0),
            DptValue::EnergyI32(1),
            DptValue::EnergyU32(1),
            DptValue::Char('A'),
            DptValue::Rgb {
                red: 1,
                green: 2,
                blue: 3,
            },
            DptValue::Rgbw {
                red: 1,
                green: 2,
                blue: 3,
                white: 4,
            },
            // DPT21/22 raw bitsets (decode-only) must also be rejected here — a
            // decoded mask can never be silently inference-written.
            DptValue::Bitset8(0xFF),
            DptValue::Bitset16(0xFFFF),
        ] {
            assert!(
                matches!(
                    encode_value(value),
                    Err(KnxIpError::Dpt(DptError::UnsupportedDpt(_)))
                ),
                "decode-only / colour variant must be refused by encode_value",
            );
        }
    }

    #[test]
    fn encode_value_still_encodes_a_writable_scalar() {
        // contrast: a genuinely writable variant still round-trips through the
        // inference path (Bool -> 1.001), so the refusal is targeted, not blanket.
        assert!(encode_value(DptValue::Bool(true)).is_ok());
    }
}

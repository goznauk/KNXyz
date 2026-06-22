//! Test-only loopback fixture for Python connected-lifecycle and
//! group-communication tests.
//!
//! Binds a scripted simulator gateway on an ephemeral localhost UDP port
//! and prints `PORT=<port>` on stdout. Except in `discovery` mode (which
//! performs no tunnel handshake), it then accepts exactly one tunnel
//! CONNECT_REQUEST (replying with a CONNECT_RESPONSE) and prints
//! `CONNECTED`. Behavior depends on the mode argument:
//!
//! - `lifecycle` (default): wait for the client's orderly
//!   DISCONNECT_REQUEST (recorded as a `DISCONNECT={...}` stdout line and
//!   answered with a DISCONNECT_RESPONSE) or stdin EOF, then exit.
//! - `discovery`: answer exactly one SEARCH_REQUEST with
//!   `--responders <n>` (default 1) SEARCH_RESPONSEs (service families
//!   core 2.1 and tunnelling 4.2). The first response advertises this
//!   gateway's own endpoint; each additional one advertises the same
//!   IP with the port incremented by its index — distinct,
//!   deterministic, fixture-ordered endpoints for multi-responder
//!   accumulation tests. Then wait for stdin EOF.
//! - `group`: serve tunnelling requests in a loop — every request is
//!   echoed to stdout as a `FRAME={...}` JSON line and acked; each
//!   GroupValueRead additionally triggers a GroupValueResponse
//!   indication carrying payload `[0x01]` (override with
//!   `--read-reply-payload <b0,b1,...>`, decimal bytes — used by the
//!   device tests for false/invalid-payload paths), addressed to the
//!   request's group unless `--read-reply-group <a/b/c>` overrides it
//!   (used by mismatch tests). An orderly DISCONNECT_REQUEST is recorded
//!   as a `DISCONNECT={...}` line and answered with a DISCONNECT_RESPONSE,
//!   ending the loop; the loop also ends when stdin reaches EOF.
//!
//! A hard 60-second safety timeout guarantees the process never outlives
//! an orphaned test run. Loopback only; never contacts a real KNX
//! installation. Spawned by `bindings/python/tests/conftest.py`; not a
//! production tool (the crate is `publish = false`).

use std::io::{BufRead, Write};
use std::time::Duration;

use knx_core::{Apci, CemiFrame, GroupAddress};
use knx_sim::{SimGateway, TunnelInbound};

const CHANNEL_ID: u8 = 0x2a;
const SAFETY_TIMEOUT: Duration = Duration::from_secs(60);
const READ_REPLY_PAYLOAD: [u8; 1] = [0x01];

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    match tokio::time::timeout(SAFETY_TIMEOUT, run()).await {
        Ok(result) => result,
        Err(_) => {
            eprintln!("knx_sim_lifecycle_fixture: safety timeout reached; exiting");
            // Hard exit: returning would hang on runtime drop, which waits
            // for the spawn_blocking stdin reader that is still waiting.
            std::process::exit(0);
        }
    }
}

struct FixtureArgs {
    mode: String,
    read_reply_group: Option<GroupAddress>,
    read_reply_payload: Vec<u8>,
    responders: u16,
    expose_group: Option<GroupAddress>,
    expose_write_payload: Option<Vec<u8>>,
}

fn parse_byte_list(value: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    Ok(value
        .split(',')
        .map(|part| part.trim().parse::<u8>())
        .collect::<Result<Vec<u8>, _>>()?)
}

fn parse_args() -> Result<FixtureArgs, Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut mode = String::from("lifecycle");
    let mut read_reply_group = None;
    let mut read_reply_payload = READ_REPLY_PAYLOAD.to_vec();
    let mut responders: u16 = 1;
    let mut expose_group = None;
    let mut expose_write_payload = None;
    let mut iter = args.into_iter();
    while let Some(arg) = iter.next() {
        if arg == "--read-reply-group" {
            let value = iter
                .next()
                .ok_or("--read-reply-group requires a group address")?;
            read_reply_group = Some(value.parse::<GroupAddress>()?);
        } else if arg == "--read-reply-payload" {
            let value = iter
                .next()
                .ok_or("--read-reply-payload requires a comma-separated byte list")?;
            read_reply_payload = parse_byte_list(&value)?;
        } else if arg == "--responders" {
            let value = iter.next().ok_or("--responders requires a count")?;
            responders = value.trim().parse::<u16>()?;
        } else if arg == "--expose-group" {
            let value = iter
                .next()
                .ok_or("--expose-group requires a group address")?;
            expose_group = Some(value.parse::<GroupAddress>()?);
        } else if arg == "--expose-write-payload" {
            let value = iter
                .next()
                .ok_or("--expose-write-payload requires a comma-separated byte list")?;
            expose_write_payload = Some(parse_byte_list(&value)?);
        } else if arg.starts_with("--") {
            // Catch typos immediately instead of treating them as a mode
            // name, which would only fail after the connect handshake.
            return Err(format!("unknown fixture flag {arg:?}").into());
        } else {
            mode = arg;
        }
    }
    Ok(FixtureArgs {
        mode,
        read_reply_group,
        read_reply_payload,
        responders,
        expose_group,
        expose_write_payload,
    })
}

fn wait_for_stdin_eof() -> tokio::task::JoinHandle<()> {
    tokio::task::spawn_blocking(|| {
        let stdin = std::io::stdin();
        let mut line = String::new();
        let _ = stdin.lock().read_line(&mut line);
    })
}

async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let FixtureArgs {
        mode,
        read_reply_group,
        read_reply_payload,
        responders,
        expose_group,
        expose_write_payload,
    } = parse_args()?;
    let mut gateway = SimGateway::bind_localhost().await?;
    let mut stdout = std::io::stdout();
    writeln!(stdout, "PORT={}", gateway.local_addr().port())?;
    stdout.flush()?;

    if mode == "discovery" {
        gateway.expect_search_request().await?;
        // One scripted gateway socket emitting N responses: the wire
        // sees N distinct SEARCH_RESPONSEs (deterministic fixture
        // order), which is what multi-responder accumulation must
        // collect within its timeout.
        let base = gateway.local_addr();
        for index in 0..responders {
            // wrapping_add: an ephemeral base port near u16::MAX must
            // not panic the fixture (review #164)
            let advertised = std::net::SocketAddr::new(base.ip(), base.port().wrapping_add(index));
            gateway
                .reply_search_response(advertised, &[(2, 1), (4, 2)])
                .await?;
        }
        writeln!(stdout, "SEARCHED")?;
        stdout.flush()?;
        wait_for_stdin_eof().await?;
        gateway.shutdown().await?;
        return Ok(());
    }

    gateway.expect_connect_request().await?;
    gateway.reply_connect_response(CHANNEL_ID).await?;
    writeln!(stdout, "CONNECTED")?;
    stdout.flush()?;

    match mode.as_str() {
        "lifecycle" => {
            // Hold the gateway socket open until the client sends an orderly
            // DISCONNECT_REQUEST or the parent signals teardown by closing our
            // stdin (or kills us).
            let stdin_eof = wait_for_stdin_eof();
            tokio::select! {
                result = serve_until_disconnect(&mut gateway) => result?,
                _ = stdin_eof => {}
            }
        }
        "group" => {
            let stdin_eof = wait_for_stdin_eof();
            tokio::select! {
                result = serve_group_requests(&mut gateway, read_reply_group, &read_reply_payload) => result?,
                _ = stdin_eof => {}
            }
        }
        "expose" => {
            let group = expose_group.ok_or("expose mode requires --expose-group")?;
            serve_expose_scenario(&mut gateway, group, expose_write_payload.as_deref()).await?;
            let stdin_eof = wait_for_stdin_eof();
            tokio::select! {
                result = serve_until_disconnect(&mut gateway) => result?,
                _ = stdin_eof => {}
            }
        }
        other => return Err(format!("unknown fixture mode {other:?}").into()),
    }

    gateway.shutdown().await?;
    Ok(())
}

/// Wait for the client's orderly DISCONNECT_REQUEST, record it for the parent
/// test, and reply with a DISCONNECT_RESPONSE.
async fn serve_until_disconnect(
    gateway: &mut SimGateway,
) -> Result<(), Box<dyn std::error::Error>> {
    gateway.expect_disconnect_request(CHANNEL_ID).await?;
    record_disconnect()?;
    gateway.reply_disconnect_response(CHANNEL_ID).await?;
    Ok(())
}

/// Emit a `DISCONNECT={...}` stdout line so the parent test can assert
/// the client sent a real DISCONNECT_REQUEST (mirrors the `FRAME=` line).
fn record_disconnect() -> std::io::Result<()> {
    let mut stdout = std::io::stdout();
    writeln!(stdout, "DISCONNECT={{\"channel_id\":{CHANNEL_ID}}}")?;
    stdout.flush()
}

/// Echo a received client tunnelling frame as a `FRAME={...}` stdout line
/// the parent test asserts on.
fn print_frame(frame: &CemiFrame) -> std::io::Result<()> {
    let mut stdout = std::io::stdout();
    let telegram = frame.telegram();
    writeln!(
        stdout,
        "FRAME={{\"dst\":\"{}\",\"apci\":\"{:?}\",\"payload\":{:?}}}",
        telegram.destination(),
        telegram.apci(),
        telegram.payload(),
    )?;
    stdout.flush()
}

/// Drive an ExposeSensor.respond_to_read scenario: optionally populate the
/// device's stored value with a wire-real GroupValueWrite indication, then
/// send a GroupValueRead and observe either the client's wire-real
/// GroupValueResponse (a `FRAME=` line) or its absence (`NO_RESPONSE`,
/// e.g. respond_to_read=False or no value observed yet).
///
/// First waits for a client-initiated probe frame (the test calls
/// ExposeSensor.sync() once it is started). That probe arrives only after
/// the client's monitor is subscribed, so the indications we send next are
/// actually delivered - broadcast events sent before subscription are
/// dropped, which is why a proactively-sending fixture needs this handshake.
async fn serve_expose_scenario(
    gateway: &mut SimGateway,
    group: GroupAddress,
    expose_write_payload: Option<&[u8]>,
) -> Result<(), Box<dyn std::error::Error>> {
    // Readiness handshake (client probe = the sync() GroupValueRead, seq 0).
    gateway.expect_tunnelling_request(0).await?;
    gateway.reply_tunnelling_ack(CHANNEL_ID, 0).await?;

    let mut gateway_sequence: u8 = 0;
    if let Some(payload) = expose_write_payload {
        gateway
            .send_tunnelling_indication(
                CHANNEL_ID,
                gateway_sequence,
                group,
                Apci::GroupValueWrite,
                payload,
            )
            .await?;
        gateway.expect_tunnelling_ack(gateway_sequence).await?;
        gateway_sequence = gateway_sequence.wrapping_add(1);
    }
    gateway
        .send_tunnelling_indication(
            CHANNEL_ID,
            gateway_sequence,
            group,
            Apci::GroupValueRead,
            &[],
        )
        .await?;
    gateway.expect_tunnelling_ack(gateway_sequence).await?;

    // The client's GroupValueResponse (if any) is its second client frame
    // (seq 1, after the probe). Bounded wait: respond or NO_RESPONSE. The
    // window is kept well under the test's post-scenario settle time so the
    // NO_RESPONSE verdict is emitted before any teardown DISCONNECT arrives.
    match tokio::time::timeout(
        Duration::from_millis(500),
        gateway.expect_tunnelling_or_disconnect(1),
    )
    .await
    {
        Ok(Ok(TunnelInbound::Tunnelling(frame))) => {
            print_frame(&frame)?;
            gateway.reply_tunnelling_ack(CHANNEL_ID, 1).await?;
        }
        Ok(Ok(TunnelInbound::Disconnect { .. })) => {
            record_disconnect()?;
            gateway.reply_disconnect_response(CHANNEL_ID).await?;
        }
        Ok(Err(error)) => return Err(error.into()),
        Err(_) => {
            let mut stdout = std::io::stdout();
            writeln!(stdout, "NO_RESPONSE")?;
            stdout.flush()?;
        }
    }
    Ok(())
}

async fn serve_group_requests(
    gateway: &mut SimGateway,
    read_reply_group: Option<GroupAddress>,
    read_reply_payload: &[u8],
) -> Result<(), Box<dyn std::error::Error>> {
    let mut client_sequence: u8 = 0;
    let mut gateway_sequence: u8 = 0;
    loop {
        let frame = match gateway
            .expect_tunnelling_or_disconnect(client_sequence)
            .await?
        {
            TunnelInbound::Tunnelling(frame) => frame,
            TunnelInbound::Disconnect { .. } => {
                record_disconnect()?;
                gateway.reply_disconnect_response(CHANNEL_ID).await?;
                return Ok(());
            }
        };
        let telegram = frame.telegram();
        print_frame(&frame)?;
        gateway
            .reply_tunnelling_ack(CHANNEL_ID, client_sequence)
            .await?;
        client_sequence = client_sequence.wrapping_add(1);

        if telegram.apci() == Apci::GroupValueRead {
            let destination = read_reply_group.unwrap_or_else(|| telegram.destination());
            gateway
                .send_tunnelling_indication(
                    CHANNEL_ID,
                    gateway_sequence,
                    destination,
                    Apci::GroupValueResponse,
                    read_reply_payload,
                )
                .await?;
            // The client acks every indication; consume it so the next
            // expect_tunnelling_request sees the next request, not the ack.
            gateway.expect_tunnelling_ack(gateway_sequence).await?;
            gateway_sequence = gateway_sequence.wrapping_add(1);
        }
    }
}

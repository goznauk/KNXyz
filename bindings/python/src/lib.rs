#![forbid(unsafe_code)]
#![allow(clippy::useless_conversion)]

use std::ffi::CString;
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use std::pin::Pin;
use std::str::FromStr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::Duration;

use knx_core::{GroupAddress, IndividualAddress};
use knx_dpt::DptValue;
use knx_ip::{
    ConnectionEvent, DiscoveryOptions, GroupEvent, RouteEvent, RouteMonitor, RouteSender,
    RoutingOptions, RoutingSendOptions, TunnelClient, TunnelOptions,
};
use knxyz::capi;
use pyo3::exceptions::{PyRuntimeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyCapsule};
use serde::Serialize;
use serde_json::Value;
use tokio::runtime::Runtime;
use tokio_stream::{Stream, StreamExt};

#[pyfunction]
fn encode_dpt_json<'py>(
    py: Python<'py>,
    dpt: &str,
    value_json: &str,
) -> PyResult<Bound<'py, PyBytes>> {
    let value: Value = serde_json::from_str(value_json).map_err(to_py_value_error)?;
    let value = dpt_value_from_json(dpt, &value)?;
    let bytes = knx_dpt::encode(dpt, value).map_err(to_py_value_error)?;

    Ok(PyBytes::new_bound(py, &bytes))
}

#[pyfunction]
fn decode_dpt_json(dpt: &str, bytes: &[u8]) -> PyResult<String> {
    let value = knx_dpt::decode(dpt, bytes).map_err(to_py_value_error)?;
    dpt_value_to_json(value)
}

#[pyfunction]
fn parse_individual_address(value: &str) -> PyResult<String> {
    Ok(IndividualAddress::from_str(value)
        .map_err(to_py_value_error)?
        .to_string())
}

#[pyfunction]
fn format_individual_address(value: &str) -> PyResult<String> {
    parse_individual_address(value)
}

#[pyfunction]
fn parse_group_address(value: &str) -> PyResult<String> {
    Ok(GroupAddress::from_str(value)
        .map_err(to_py_value_error)?
        .to_string())
}

#[pyfunction]
fn format_group_address(value: &str) -> PyResult<String> {
    parse_group_address(value)
}

// Raw/canonical address conversions so Python address wrappers do not implement
// KNX address arithmetic themselves.
// `from_raw` is infallible (every u16 is a structurally valid address);
// `to_raw` validates the string exactly as the existing parsers do.

#[pyfunction]
fn group_address_to_raw(value: &str) -> PyResult<u16> {
    Ok(GroupAddress::from_str(value)
        .map_err(to_py_value_error)?
        .raw())
}

#[pyfunction]
fn group_address_from_raw(raw: u16) -> PyResult<String> {
    Ok(GroupAddress::from_raw(raw).to_string())
}

#[pyfunction]
fn individual_address_to_raw(value: &str) -> PyResult<u16> {
    Ok(IndividualAddress::from_str(value)
        .map_err(to_py_value_error)?
        .raw())
}

#[pyfunction]
fn individual_address_from_raw(raw: u16) -> PyResult<String> {
    Ok(IndividualAddress::from_raw(raw).to_string())
}

#[pyfunction]
fn discover_gateways_json(py: Python<'_>, options_json: &str) -> PyResult<String> {
    let options = parse_discovery_options(options_json)?;
    let gateways = py.allow_threads(|| {
        let runtime = Runtime::new().map_err(to_py_runtime_error)?;
        runtime
            .block_on(knx_ip::discover_gateways(options))
            .map_err(to_py_runtime_error)
    })?;
    let gateways = gateways
        .into_iter()
        .map(|gateway| GatewayDto {
            control_endpoint: gateway.control_endpoint.to_string(),
            received_from: gateway.received_from.to_string(),
            service_families: gateway
                .service_families
                .into_iter()
                .map(|family| ServiceFamilyDto {
                    id: family.id,
                    version: family.version,
                })
                .collect(),
        })
        .collect::<Vec<_>>();

    serde_json::to_string(&gateways).map_err(to_py_runtime_error)
}

type MonitorStream = Pin<Box<dyn Stream<Item = Result<GroupEvent, knx_ip::KnxIpError>> + Send>>;

#[pyclass]
struct NativeTunnelClient {
    runtime: Runtime,
    inner: Mutex<Option<TunnelClient>>,
    monitor: Mutex<Option<MonitorStream>>,
    monitor_waiting: AtomicBool,
}

fn closed_tunnel_error() -> PyErr {
    PyRuntimeError::new_err("tunnel client is closed")
}

fn group_event_to_json(event: &GroupEvent) -> Value {
    let apci = match event.apci {
        knx_core::Apci::GroupValueRead => "group_value_read",
        knx_core::Apci::GroupValueResponse => "group_value_response",
        knx_core::Apci::GroupValueWrite => "group_value_write",
    };
    serde_json::json!({
        "source": event.source.to_string(),
        "destination": event.destination.to_string(),
        "apci": apci,
        "payload": event.payload,
    })
}

fn lifecycle_event_to_json(event: &ConnectionEvent) -> Value {
    match event {
        ConnectionEvent::Connected { channel_id } => serde_json::json!({
            "type": "connected",
            "channel_id": channel_id,
        }),
        ConnectionEvent::Disconnected => serde_json::json!({
            "type": "disconnected",
        }),
        ConnectionEvent::Reconnecting { attempt, delay } => serde_json::json!({
            "type": "reconnecting",
            "attempt": attempt,
            "delay_ms": delay.as_millis() as u64,
        }),
        ConnectionEvent::Reconnected {
            attempt,
            channel_id,
        } => serde_json::json!({
            "type": "reconnected",
            "attempt": attempt,
            "channel_id": channel_id,
        }),
    }
}

#[pymethods]
impl NativeTunnelClient {
    #[staticmethod]
    fn connect(py: Python<'_>, options_json: &str) -> PyResult<Self> {
        let options: Value = serde_json::from_str(options_json).map_err(to_py_value_error)?;
        let options = parse_tunnel_options(&options)?;
        let (runtime, client) = py.allow_threads(|| {
            let runtime = Runtime::new().map_err(to_py_runtime_error)?;
            let client = runtime
                .block_on(TunnelClient::connect_with_options(options))
                .map_err(to_py_runtime_error)?;
            Ok::<_, PyErr>((runtime, client))
        })?;

        Ok(Self {
            runtime,
            inner: Mutex::new(Some(client)),
            monitor: Mutex::new(None),
            monitor_waiting: AtomicBool::new(false),
        })
    }

    fn write(&self, py: Python<'_>, group: &str, dpt: &str, value_json: &str) -> PyResult<()> {
        let group = GroupAddress::from_str(group).map_err(to_py_value_error)?;
        let value: Value = serde_json::from_str(value_json).map_err(to_py_value_error)?;
        let value = dpt_value_from_json(dpt, &value)?;
        py.allow_threads(|| {
            let mut client = self
                .inner
                .lock()
                .map_err(|_| PyRuntimeError::new_err("tunnel client lock poisoned"))?;
            let client = client.as_mut().ok_or_else(closed_tunnel_error)?;

            self.runtime
                .block_on(client.group_write(group, value))
                .map_err(to_py_runtime_error)
        })
    }

    /// Send a GroupValueRead request without waiting for a response
    /// (fire-and-forget; completes after the tunnelling ack). Any
    /// answer arrives as a bus indication via the monitor stream.
    fn read_request(&self, py: Python<'_>, group: &str) -> PyResult<()> {
        let group = GroupAddress::from_str(group).map_err(to_py_value_error)?;
        py.allow_threads(|| {
            let mut client = self
                .inner
                .lock()
                .map_err(|_| PyRuntimeError::new_err("tunnel client lock poisoned"))?;
            let client = client.as_mut().ok_or_else(closed_tunnel_error)?;

            self.runtime
                .block_on(client.group_read_request(group))
                .map_err(to_py_runtime_error)
        })
    }

    /// Send a GroupValueResponse telegram (answering a group read).
    fn respond(&self, py: Python<'_>, group: &str, dpt: &str, value_json: &str) -> PyResult<()> {
        let group = GroupAddress::from_str(group).map_err(to_py_value_error)?;
        let value: Value = serde_json::from_str(value_json).map_err(to_py_value_error)?;
        let value = dpt_value_from_json(dpt, &value)?;
        py.allow_threads(|| {
            let mut client = self
                .inner
                .lock()
                .map_err(|_| PyRuntimeError::new_err("tunnel client lock poisoned"))?;
            let client = client.as_mut().ok_or_else(closed_tunnel_error)?;

            self.runtime
                .block_on(client.group_response(group, value))
                .map_err(to_py_runtime_error)
        })
    }

    fn read(&self, py: Python<'_>, group: &str, dpt: &str, timeout_ms: u64) -> PyResult<String> {
        let group = GroupAddress::from_str(group).map_err(to_py_value_error)?;
        let value = py.allow_threads(|| {
            let mut client = self
                .inner
                .lock()
                .map_err(|_| PyRuntimeError::new_err("tunnel client lock poisoned"))?;
            let client = client.as_mut().ok_or_else(closed_tunnel_error)?;
            self.runtime
                .block_on(client.group_read(group, dpt, Duration::from_millis(timeout_ms)))
                .map_err(to_py_runtime_error)
        })?;

        dpt_value_to_json(value)
    }

    /// Best-effort orderly shutdown.
    ///
    /// When connected, sends `DISCONNECT_REQUEST` so the gateway can free the
    /// tunnel slot. The request is timeout-bounded and runs without the GIL; a
    /// missing or late response does not fail teardown. Closing twice is a no-op.
    fn close(&self, py: Python<'_>) -> PyResult<()> {
        py.allow_threads(|| {
            let mut client = self
                .inner
                .lock()
                .map_err(|_| PyRuntimeError::new_err("tunnel client lock poisoned"))?;
            if let Some(connected) = client.as_ref() {
                // Best-effort orderly disconnect: a silent gateway must not
                // block or fail local teardown, so the result is ignored
                // (the request was still sent on the wire).
                let _ = self.runtime.block_on(connected.disconnect());
            }
            client.take();
            let mut monitor = self
                .monitor
                .lock()
                .map_err(|_| PyRuntimeError::new_err("monitor lock poisoned"))?;
            monitor.take();
            Ok(())
        })
    }

    fn is_closed(&self, py: Python<'_>) -> PyResult<bool> {
        py.allow_threads(|| {
            let client = self
                .inner
                .lock()
                .map_err(|_| PyRuntimeError::new_err("tunnel client lock poisoned"))?;
            Ok(client.is_none())
        })
    }

    /// Recorded connection lifecycle events as a JSON array of objects:
    /// {"type": "connected", "channel_id": n} | {"type": "disconnected"} |
    /// {"type": "reconnecting", "attempt": n, "delay_ms": n} |
    /// {"type": "reconnected", "attempt": n, "channel_id": n}.
    fn lifecycle_events_json(&self, py: Python<'_>) -> PyResult<String> {
        py.allow_threads(|| {
            let client = self
                .inner
                .lock()
                .map_err(|_| PyRuntimeError::new_err("tunnel client lock poisoned"))?;
            let client = client.as_ref().ok_or_else(closed_tunnel_error)?;
            let events: Vec<Value> = client
                .lifecycle_events()
                .iter()
                .map(lifecycle_event_to_json)
                .collect();
            serde_json::to_string(&events).map_err(to_py_runtime_error)
        })
    }

    /// Subscribe to incoming group telegrams (bus indications).
    ///
    /// Events arriving before this call are not delivered (broadcast
    /// semantics). Errors if the client is closed or a monitor is
    /// already started.
    fn monitor_start(&self, py: Python<'_>) -> PyResult<()> {
        py.allow_threads(|| {
            let mut client = self
                .inner
                .lock()
                .map_err(|_| PyRuntimeError::new_err("tunnel client lock poisoned"))?;
            let client = client.as_mut().ok_or_else(closed_tunnel_error)?;
            let mut monitor = self
                .monitor
                .lock()
                .map_err(|_| PyRuntimeError::new_err("monitor lock poisoned"))?;
            if monitor.is_some() || self.monitor_waiting.load(Ordering::SeqCst) {
                return Err(PyRuntimeError::new_err("monitor already started"));
            }
            *monitor = Some(Box::pin(client.monitor()));
            Ok(())
        })
    }

    fn monitor_started(&self, py: Python<'_>) -> PyResult<bool> {
        py.allow_threads(|| {
            let monitor = self
                .monitor
                .lock()
                .map_err(|_| PyRuntimeError::new_err("monitor lock poisoned"))?;
            Ok(monitor.is_some() || self.monitor_waiting.load(Ordering::SeqCst))
        })
    }

    /// Block until the next incoming group telegram, the timeout, or an
    /// error. Returns a JSON object {"source", "destination", "apci",
    /// "payload"} with apci one of "group_value_read" /
    /// "group_value_response" / "group_value_write". A lagged monitor
    /// (events dropped because the consumer fell behind) raises rather
    /// than silently skipping.
    fn monitor_next_json(&self, py: Python<'_>, timeout_ms: u64) -> PyResult<String> {
        // The whole body is GIL-free: every section below either waits
        // or acquires a mutex that another thread may hold across a
        // GIL-released wait.
        py.allow_threads(|| {
            {
                let client = self
                    .inner
                    .lock()
                    .map_err(|_| PyRuntimeError::new_err("tunnel client lock poisoned"))?;
                if client.is_none() {
                    return Err(closed_tunnel_error());
                }
            }
            // Take the stream out of the mutex for the duration of the wait
            // so close()/monitor_started()/monitor_start() never block behind
            // a pending monitor_next; monitor_waiting keeps the subscription
            // observable and prevents concurrent consumers.
            let mut stream = {
                let mut monitor = self
                    .monitor
                    .lock()
                    .map_err(|_| PyRuntimeError::new_err("monitor lock poisoned"))?;
                match monitor.take() {
                    Some(stream) => {
                        self.monitor_waiting.store(true, Ordering::SeqCst);
                        stream
                    }
                    None if self.monitor_waiting.load(Ordering::SeqCst) => {
                        return Err(PyRuntimeError::new_err(
                            "monitor busy: another monitor_next is pending",
                        ));
                    }
                    None => return Err(PyRuntimeError::new_err("monitor not started")),
                }
            };
            let outcome = self.runtime.block_on(async {
                tokio::time::timeout(Duration::from_millis(timeout_ms), stream.next()).await
            });
            // Put the subscription back before clearing the waiting flag
            // unless the client was closed while we waited (close()
            // cleared the slot; do not resurrect it).
            let closed = {
                let client = self
                    .inner
                    .lock()
                    .map_err(|_| PyRuntimeError::new_err("tunnel client lock poisoned"))?;
                client.is_none()
            };
            if closed {
                drop(stream);
                self.monitor_waiting.store(false, Ordering::SeqCst);
                return Err(closed_tunnel_error());
            }
            {
                let mut monitor = self
                    .monitor
                    .lock()
                    .map_err(|_| PyRuntimeError::new_err("monitor lock poisoned"))?;
                *monitor = Some(stream);
            }
            self.monitor_waiting.store(false, Ordering::SeqCst);
            match outcome {
                Err(_) => Err(PyRuntimeError::new_err(
                    "monitor timeout: no telegram received",
                )),
                Ok(None) => Err(PyRuntimeError::new_err("monitor stream ended")),
                Ok(Some(Err(error))) => Err(to_py_runtime_error(error)),
                Ok(Some(Ok(event))) => {
                    serde_json::to_string(&group_event_to_json(&event)).map_err(to_py_runtime_error)
                }
            }
        })
    }
}

type RouteEventStream = Pin<Box<dyn Stream<Item = Result<RouteEvent, knx_ip::KnxIpError>> + Send>>;

fn closed_route_error() -> PyErr {
    PyRuntimeError::new_err("routing client is closed")
}

fn route_event_to_json(event: &RouteEvent) -> Value {
    let apci = match event.apci {
        knx_core::Apci::GroupValueRead => "group_value_read",
        knx_core::Apci::GroupValueResponse => "group_value_response",
        knx_core::Apci::GroupValueWrite => "group_value_write",
    };
    serde_json::json!({
        "source": event.source.to_string(),
        "destination": event.destination.to_string(),
        "apci": apci,
        "payload": event.payload,
        "peer": event.peer.to_string(),
    })
}

/// Parse a routing options JSON object into (send, receive) option
/// structs and the source individual address. `loopback`/`local_ip`
/// (interface) keep test traffic on the loopback NIC; `ttl` defaults to
/// 1 so a real send never escapes the local segment.
fn parse_routing_options(
    value: &Value,
) -> PyResult<(RoutingSendOptions, RoutingOptions, IndividualAddress)> {
    let group: Ipv4Addr = match value.get("multicast_group").and_then(Value::as_str) {
        Some(text) => text.parse().map_err(to_py_value_error)?,
        None => Ipv4Addr::new(224, 0, 23, 12),
    };
    if !group.is_multicast() {
        return Err(PyValueError::new_err(
            "routing requires an IPv4 multicast group (224.0.0.0/4)",
        ));
    }
    let port = value
        .get("multicast_port")
        .and_then(Value::as_u64)
        .unwrap_or(3671);
    let port =
        u16::try_from(port).map_err(|_| PyValueError::new_err("multicast_port is out of range"))?;
    let interface: Option<Ipv4Addr> = match value
        .get("local_ip")
        .or_else(|| value.get("interface"))
        .and_then(Value::as_str)
    {
        Some(text) => Some(text.parse().map_err(to_py_value_error)?),
        None => None,
    };
    let loopback = value
        .get("loopback")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let ttl = value.get("ttl").and_then(Value::as_u64).unwrap_or(1);
    let ttl = u32::try_from(ttl).map_err(|_| PyValueError::new_err("ttl is out of range"))?;
    let source: IndividualAddress = value
        .get("source")
        .or_else(|| value.get("individual_address"))
        .and_then(Value::as_str)
        .unwrap_or("0.0.0")
        .parse()
        .map_err(to_py_value_error)?;
    let recv_bind: SocketAddrV4 = match value.get("bind").and_then(Value::as_str) {
        Some(text) => text.parse().map_err(to_py_value_error)?,
        None => SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, port),
    };

    let send_options = RoutingSendOptions {
        target: SocketAddrV4::new(group, port),
        interface,
        loopback,
        ttl,
        ..RoutingSendOptions::default()
    };
    let recv_options = RoutingOptions {
        bind: recv_bind,
        multicast: group,
        port,
        interface,
        loopback,
    };
    Ok((send_options, recv_options, source))
}

/// Native KNXnet/IP ROUTING client (connectionless UDP multicast).
///
/// Mirrors `NativeTunnelClient`: `send` multicasts a GroupValueWrite
/// RoutingIndication, the lazily-started monitor delivers wire-real
/// RouteEvents, and `close` drops the sockets (routing is connectionless
/// — there is no disconnect frame). Every blocking call runs under
/// `py.allow_threads` (the GIL-hygiene rule).
#[pyclass]
struct NativeRouteClient {
    runtime: Runtime,
    sender: Mutex<Option<RouteSender>>,
    monitor: Mutex<Option<RouteEventStream>>,
    monitor_waiting: AtomicBool,
    source: IndividualAddress,
    recv_options: RoutingOptions,
}

#[pymethods]
impl NativeRouteClient {
    #[staticmethod]
    fn connect(py: Python<'_>, options_json: &str) -> PyResult<Self> {
        let options: Value = serde_json::from_str(options_json).map_err(to_py_value_error)?;
        let (send_options, recv_options, source) = parse_routing_options(&options)?;
        let (runtime, sender) = py.allow_threads(|| {
            let runtime = Runtime::new().map_err(to_py_runtime_error)?;
            let sender = runtime
                .block_on(RouteSender::bind(send_options))
                .map_err(to_py_runtime_error)?;
            Ok::<_, PyErr>((runtime, sender))
        })?;

        Ok(Self {
            runtime,
            sender: Mutex::new(Some(sender)),
            monitor: Mutex::new(None),
            monitor_waiting: AtomicBool::new(false),
            source,
            recv_options,
        })
    }

    /// Multicast a GroupValueWrite RoutingIndication for `value`.
    fn send(&self, py: Python<'_>, group: &str, dpt: &str, value_json: &str) -> PyResult<usize> {
        let group = GroupAddress::from_str(group).map_err(to_py_value_error)?;
        let value: Value = serde_json::from_str(value_json).map_err(to_py_value_error)?;
        let value = dpt_value_from_json(dpt, &value)?;
        py.allow_threads(|| {
            let sender = self
                .sender
                .lock()
                .map_err(|_| PyRuntimeError::new_err("routing client lock poisoned"))?;
            let sender = sender.as_ref().ok_or_else(closed_route_error)?;
            self.runtime
                .block_on(sender.send_group_write(self.source, group, value))
                .map_err(to_py_runtime_error)
        })
    }

    /// Join the multicast group and start receiving RoutingIndications.
    fn monitor_start(&self, py: Python<'_>) -> PyResult<()> {
        py.allow_threads(|| {
            {
                let sender = self
                    .sender
                    .lock()
                    .map_err(|_| PyRuntimeError::new_err("routing client lock poisoned"))?;
                if sender.is_none() {
                    return Err(closed_route_error());
                }
            }
            let mut monitor = self
                .monitor
                .lock()
                .map_err(|_| PyRuntimeError::new_err("monitor lock poisoned"))?;
            if monitor.is_some() || self.monitor_waiting.load(Ordering::SeqCst) {
                return Err(PyRuntimeError::new_err("monitor already started"));
            }
            let route_monitor = self
                .runtime
                .block_on(RouteMonitor::bind(self.recv_options))
                .map_err(to_py_runtime_error)?;
            *monitor = Some(Box::pin(route_monitor.events()));
            Ok(())
        })
    }

    fn monitor_started(&self, py: Python<'_>) -> PyResult<bool> {
        py.allow_threads(|| {
            let monitor = self
                .monitor
                .lock()
                .map_err(|_| PyRuntimeError::new_err("monitor lock poisoned"))?;
            Ok(monitor.is_some() || self.monitor_waiting.load(Ordering::SeqCst))
        })
    }

    /// Block until the next RoutingIndication, the timeout, or an error.
    /// Returns {"source","destination","apci","payload","peer"}.
    fn monitor_next_json(&self, py: Python<'_>, timeout_ms: u64) -> PyResult<String> {
        py.allow_threads(|| {
            {
                let sender = self
                    .sender
                    .lock()
                    .map_err(|_| PyRuntimeError::new_err("routing client lock poisoned"))?;
                if sender.is_none() {
                    return Err(closed_route_error());
                }
            }
            let mut stream = {
                let mut monitor = self
                    .monitor
                    .lock()
                    .map_err(|_| PyRuntimeError::new_err("monitor lock poisoned"))?;
                match monitor.take() {
                    Some(stream) => {
                        self.monitor_waiting.store(true, Ordering::SeqCst);
                        stream
                    }
                    None if self.monitor_waiting.load(Ordering::SeqCst) => {
                        return Err(PyRuntimeError::new_err(
                            "monitor busy: another monitor_next is pending",
                        ));
                    }
                    None => return Err(PyRuntimeError::new_err("monitor not started")),
                }
            };
            let outcome = self.runtime.block_on(async {
                tokio::time::timeout(Duration::from_millis(timeout_ms), stream.next()).await
            });
            let closed = {
                let sender = self
                    .sender
                    .lock()
                    .map_err(|_| PyRuntimeError::new_err("routing client lock poisoned"))?;
                sender.is_none()
            };
            if closed {
                drop(stream);
                self.monitor_waiting.store(false, Ordering::SeqCst);
                return Err(closed_route_error());
            }
            {
                let mut monitor = self
                    .monitor
                    .lock()
                    .map_err(|_| PyRuntimeError::new_err("monitor lock poisoned"))?;
                *monitor = Some(stream);
            }
            self.monitor_waiting.store(false, Ordering::SeqCst);
            match outcome {
                Err(_) => Err(PyRuntimeError::new_err(
                    "monitor timeout: no telegram received",
                )),
                Ok(None) => Err(PyRuntimeError::new_err("monitor stream ended")),
                Ok(Some(Err(error))) => Err(to_py_runtime_error(error)),
                Ok(Some(Ok(event))) => {
                    serde_json::to_string(&route_event_to_json(&event)).map_err(to_py_runtime_error)
                }
            }
        })
    }

    /// Release the routing sockets. Idempotent; routing is connectionless
    /// so there is no disconnect frame to send.
    fn close(&self, py: Python<'_>) -> PyResult<()> {
        py.allow_threads(|| {
            let mut sender = self
                .sender
                .lock()
                .map_err(|_| PyRuntimeError::new_err("routing client lock poisoned"))?;
            sender.take();
            let mut monitor = self
                .monitor
                .lock()
                .map_err(|_| PyRuntimeError::new_err("monitor lock poisoned"))?;
            monitor.take();
            Ok(())
        })
    }

    fn is_closed(&self, py: Python<'_>) -> PyResult<bool> {
        py.allow_threads(|| {
            let sender = self
                .sender
                .lock()
                .map_err(|_| PyRuntimeError::new_err("routing client lock poisoned"))?;
            Ok(sender.is_none())
        })
    }
}

#[pymodule]
fn _knxyz(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(encode_dpt_json, m)?)?;
    m.add_function(wrap_pyfunction!(decode_dpt_json, m)?)?;
    m.add_function(wrap_pyfunction!(parse_individual_address, m)?)?;
    m.add_function(wrap_pyfunction!(format_individual_address, m)?)?;
    m.add_function(wrap_pyfunction!(parse_group_address, m)?)?;
    m.add_function(wrap_pyfunction!(format_group_address, m)?)?;
    m.add_function(wrap_pyfunction!(group_address_to_raw, m)?)?;
    m.add_function(wrap_pyfunction!(group_address_from_raw, m)?)?;
    m.add_function(wrap_pyfunction!(individual_address_to_raw, m)?)?;
    m.add_function(wrap_pyfunction!(individual_address_from_raw, m)?)?;
    m.add_function(wrap_pyfunction!(discover_gateways_json, m)?)?;
    m.add_class::<NativeTunnelClient>()?;
    m.add_class::<NativeRouteClient>()?;
    let capsule_name =
        CString::new("knxyz._knxyz._C_API").expect("static capsule name contains no nul bytes");
    m.add(
        "_C_API",
        PyCapsule::new_bound(m.py(), capi::capi_v1(), Some(capsule_name))?,
    )?;

    Ok(())
}

#[derive(Serialize)]
struct GatewayDto {
    #[serde(rename = "controlEndpoint")]
    control_endpoint: String,
    #[serde(rename = "receivedFrom")]
    received_from: String,
    #[serde(rename = "serviceFamilies")]
    service_families: Vec<ServiceFamilyDto>,
}

#[derive(Serialize)]
struct ServiceFamilyDto {
    id: u8,
    version: u8,
}

fn parse_discovery_options(input: &str) -> PyResult<DiscoveryOptions> {
    let value: Value = serde_json::from_str(input).map_err(to_py_value_error)?;
    let mut options = DiscoveryOptions::default();

    if let Some(bind) = value.get("bind").and_then(Value::as_str) {
        options.bind = bind.parse().map_err(to_py_value_error)?;
    }
    if let Some(target) = value.get("target").and_then(Value::as_str) {
        options.target = target.parse().map_err(to_py_value_error)?;
    }
    if let Some(timeout_ms) = value.get("timeout_ms").and_then(Value::as_u64) {
        options.timeout = Duration::from_millis(timeout_ms);
    }

    Ok(options)
}

fn tunnel_target_from_json(value: &Value) -> PyResult<SocketAddr> {
    if let Some(target) = value.get("target").and_then(Value::as_str) {
        return target.parse().map_err(to_py_value_error);
    }

    let host = value
        .get("host")
        .and_then(Value::as_str)
        .ok_or_else(|| PyValueError::new_err("connect_tunnel requires host or target"))?;
    let port = value.get("port").and_then(Value::as_u64).unwrap_or(3671);
    let port = u16::try_from(port)
        .map_err(|_| PyValueError::new_err("connect_tunnel port is out of range"))?;

    format!("{host}:{port}").parse().map_err(to_py_value_error)
}

fn parse_tunnel_options(value: &Value) -> PyResult<TunnelOptions> {
    let target = tunnel_target_from_json(value)?;
    let mut options = TunnelOptions::new(target);

    if let Some(bind) = value.get("bind").and_then(Value::as_str) {
        options.bind = bind.parse().map_err(to_py_value_error)?;
    }
    if let Some(control_endpoint) = value.get("control_endpoint").and_then(Value::as_str) {
        options.control_endpoint = Some(control_endpoint.parse().map_err(to_py_value_error)?);
    }
    if let Some(data_endpoint) = value.get("data_endpoint").and_then(Value::as_str) {
        options.data_endpoint = Some(data_endpoint.parse().map_err(to_py_value_error)?);
    }
    if let Some(ack_timeout_ms) = value.get("ack_timeout_ms").and_then(Value::as_u64) {
        options.ack_timeout = Duration::from_millis(ack_timeout_ms);
    }

    Ok(options)
}

fn dpt_value_from_json(dpt: &str, value: &Value) -> PyResult<DptValue> {
    if let Some(value) = dpt_value_from_typed_json(value)? {
        return Ok(value);
    }
    if dpt.starts_with("1.") {
        return json_bool(value).map(DptValue::Bool);
    }
    if dpt == "5.001" {
        return json_f32(value).map(DptValue::Scaling);
    }
    if dpt.starts_with("5.") {
        return json_u8(value).map(DptValue::U8);
    }
    if dpt == "9.001" {
        return json_f32(value).map(DptValue::Temperature);
    }
    if dpt == "17.001" {
        // DPT 17.001 scene number: the on-bus field is 0-63 (the native
        // codec validates the upper bound). The 1-based scene-number
        // API convenience lives in the Scene facade, not here.
        return json_u8(value).map(DptValue::SceneNumber);
    }

    Err(PyValueError::new_err(format!(
        "unsupported DPT for Python binding: {dpt}"
    )))
}

fn dpt_value_from_typed_json(value: &Value) -> PyResult<Option<DptValue>> {
    let Some(kind) = value.get("type").and_then(Value::as_str) else {
        return Ok(None);
    };

    let value = match kind {
        "bool" => DptValue::Bool(json_bool(json_field(value, "value")?)?),
        "u8" => DptValue::U8(json_u8(json_field(value, "value")?)?),
        "scaling" => DptValue::Scaling(json_f32(json_field(value, "value")?)?),
        "temperature" => DptValue::Temperature(json_f32(json_field(value, "value")?)?),
        "control_bool" => DptValue::ControlBool {
            control: json_bool(json_field(value, "control")?)?,
            value: json_bool(json_field(value, "value")?)?,
        },
        "step_control" => DptValue::StepControl {
            increase: json_bool(json_field(value, "increase")?)?,
            step_code: json_u8(json_field(value, "step_code")?)?,
        },
        "i8" => DptValue::I8(json_i8(json_field(value, "value")?)?),
        "u16" => DptValue::U16(json_u16(json_field(value, "value")?)?),
        "i16" => DptValue::I16(json_i16(json_field(value, "value")?)?),
        "time" => DptValue::Time {
            weekday: json_u8(json_field(value, "weekday")?)?,
            hour: json_u8(json_field(value, "hour")?)?,
            minute: json_u8(json_field(value, "minute")?)?,
            second: json_u8(json_field(value, "second")?)?,
        },
        "date" => DptValue::Date {
            year: json_u16(json_field(value, "year")?)?,
            month: json_u8(json_field(value, "month")?)?,
            day: json_u8(json_field(value, "day")?)?,
        },
        "datetime" => DptValue::DateTime {
            year: json_u16(json_field(value, "year")?)?,
            month: json_u8(json_field(value, "month")?)?,
            day: json_u8(json_field(value, "day")?)?,
            weekday: json_u8(json_field(value, "weekday")?)?,
            hour: json_u8(json_field(value, "hour")?)?,
            minute: json_u8(json_field(value, "minute")?)?,
            second: json_u8(json_field(value, "second")?)?,
        },
        "u32" => DptValue::U32(json_u32(json_field(value, "value")?)?),
        "i32" => DptValue::I32(json_i32(json_field(value, "value")?)?),
        "f32" => DptValue::F32(json_f32(json_field(value, "value")?)?),
        "text14" => DptValue::Text14(json_string(json_field(value, "value")?)?),
        "scene_number" => DptValue::SceneNumber(json_u8(json_field(value, "value")?)?),
        "scene_control" => DptValue::SceneControl {
            learn: json_bool(json_field(value, "learn")?)?,
            scene: json_u8(json_field(value, "scene")?)?,
        },
        "rgb" => DptValue::Rgb {
            red: json_u8(json_field(value, "red")?)?,
            green: json_u8(json_field(value, "green")?)?,
            blue: json_u8(json_field(value, "blue")?)?,
        },
        "rgbw" => DptValue::Rgbw {
            red: json_u8(json_field(value, "red")?)?,
            green: json_u8(json_field(value, "green")?)?,
            blue: json_u8(json_field(value, "blue")?)?,
            white: json_u8(json_field(value, "white")?)?,
        },
        "hvac_mode" => DptValue::HvacMode(json_u8(json_field(value, "value")?)?),
        "hvac_controller_mode" => {
            DptValue::HvacControllerMode(json_u8(json_field(value, "value")?)?)
        }
        "energy_i32" => DptValue::EnergyI32(json_i32(json_field(value, "value")?)?),
        "energy_u32" => DptValue::EnergyU32(json_u32(json_field(value, "value")?)?),
        // i64 (DPT29 V64) crosses JSON as a decimal string so values outside
        // JavaScript's safe integer range are preserved exactly.
        "i64" => DptValue::I64(json_i64_str(json_field(value, "value")?)?),
        // char (DPT4) crosses JSON as a 1-char string.
        "char" => DptValue::Char(json_char(json_field(value, "value")?)?),
        // DPT21/22 raw bitsets cross JSON as plain u8/u16 numbers. from-json is
        // for round-trip/marshal only: encode still refuses because mains 21/22
        // are absent from the codec table and Bitset* is rejected by encode_value.
        "bitset8" => DptValue::Bitset8(json_u8(json_field(value, "value")?)?),
        "bitset16" => DptValue::Bitset16(json_u16(json_field(value, "value")?)?),
        _ => {
            return Err(PyValueError::new_err(format!(
                "unsupported DPT JSON value type: {kind}"
            )));
        }
    };

    Ok(Some(value))
}

fn dpt_value_to_json(value: DptValue) -> PyResult<String> {
    let value = match value {
        DptValue::Bool(value) => serde_json::json!(value),
        DptValue::U8(value) => serde_json::json!(value),
        // Float16 (weather 9.004/5/6/7) and Angle (5.003) decode to a plain
        // JSON number, exactly like Temperature/Scaling — the number carries
        // no unit tag, so non-temperature weather values are not mislabeled
        // (the DPT id at the call site carries the unit). Decode-only.
        DptValue::Scaling(value)
        | DptValue::Temperature(value)
        | DptValue::Float16(value)
        | DptValue::Angle(value) => serde_json::json!(value),
        DptValue::ControlBool { control, value } => {
            serde_json::json!({ "type": "control_bool", "control": control, "value": value })
        }
        DptValue::StepControl {
            increase,
            step_code,
        } => {
            serde_json::json!({ "type": "step_control", "increase": increase, "step_code": step_code })
        }
        DptValue::I8(value) => serde_json::json!({ "type": "i8", "value": value }),
        DptValue::U16(value) => serde_json::json!({ "type": "u16", "value": value }),
        DptValue::I16(value) => serde_json::json!({ "type": "i16", "value": value }),
        DptValue::Time {
            weekday,
            hour,
            minute,
            second,
        } => {
            serde_json::json!({ "type": "time", "weekday": weekday, "hour": hour, "minute": minute, "second": second })
        }
        DptValue::Date { year, month, day } => {
            serde_json::json!({ "type": "date", "year": year, "month": month, "day": day })
        }
        DptValue::DateTime {
            year,
            month,
            day,
            weekday,
            hour,
            minute,
            second,
        } => {
            serde_json::json!({ "type": "datetime", "year": year, "month": month, "day": day, "weekday": weekday, "hour": hour, "minute": minute, "second": second })
        }
        DptValue::U32(value) => serde_json::json!({ "type": "u32", "value": value }),
        DptValue::I32(value) => serde_json::json!({ "type": "i32", "value": value }),
        DptValue::F32(value) => serde_json::json!({ "type": "f32", "value": value }),
        DptValue::Text14(value) => serde_json::json!({ "type": "text14", "value": value }),
        DptValue::SceneNumber(value) => serde_json::json!(value),
        DptValue::SceneControl { learn, scene } => {
            serde_json::json!({ "type": "scene_control", "learn": learn, "scene": scene })
        }
        DptValue::Rgb { red, green, blue } => {
            serde_json::json!({ "type": "rgb", "red": red, "green": green, "blue": blue })
        }
        DptValue::Rgbw {
            red,
            green,
            blue,
            white,
        } => {
            serde_json::json!({ "type": "rgbw", "red": red, "green": green, "blue": blue, "white": white })
        }
        DptValue::HvacMode(value) => serde_json::json!({ "type": "hvac_mode", "value": value }),
        DptValue::HvacControllerMode(value) => {
            serde_json::json!({ "type": "hvac_controller_mode", "value": value })
        }
        DptValue::EnergyI32(value) => {
            serde_json::json!({ "type": "energy_i32", "value": value })
        }
        DptValue::EnergyU32(value) => {
            serde_json::json!({ "type": "energy_u32", "value": value })
        }
        // i64 (DPT29 V64) is emitted as a decimal string so values outside
        // JavaScript's safe integer range are preserved exactly. Python ints are
        // arbitrary-precision, but the same string shape is used in both bindings.
        DptValue::I64(value) => {
            serde_json::json!({ "type": "i64", "value": value.to_string() })
        }
        // char (DPT4) is emitted as a 1-char string.
        DptValue::Char(value) => {
            serde_json::json!({ "type": "char", "value": value.to_string() })
        }
        // DPT21/22 raw bitsets cross JSON as plain numbers (u8/u16 fit the JS
        // safe-integer range, unlike i64 above which needs a decimal string).
        DptValue::Bitset8(value) => {
            serde_json::json!({ "type": "bitset8", "value": value })
        }
        DptValue::Bitset16(value) => {
            serde_json::json!({ "type": "bitset16", "value": value })
        }
    };

    serde_json::to_string(&value).map_err(to_py_runtime_error)
}

fn json_field<'a>(value: &'a Value, field: &str) -> PyResult<&'a Value> {
    value
        .get(field)
        .ok_or_else(|| PyValueError::new_err(format!("expected DPT JSON field: {field}")))
}

fn json_bool(value: &Value) -> PyResult<bool> {
    if let Some(value) = value.as_bool() {
        return Ok(value);
    }
    match value.as_u64() {
        Some(0) => Ok(false),
        Some(1) => Ok(true),
        _ => Err(PyValueError::new_err("expected boolean DPT value")),
    }
}

fn json_i8(value: &Value) -> PyResult<i8> {
    let value = value
        .as_i64()
        .ok_or_else(|| PyValueError::new_err("expected signed DPT value"))?;

    i8::try_from(value).map_err(|_| PyValueError::new_err("DPT value is out of i8 range"))
}

fn json_u8(value: &Value) -> PyResult<u8> {
    let value = value
        .as_u64()
        .ok_or_else(|| PyValueError::new_err("expected unsigned DPT value"))?;

    u8::try_from(value).map_err(|_| PyValueError::new_err("DPT value is out of u8 range"))
}

fn json_u16(value: &Value) -> PyResult<u16> {
    let value = value
        .as_u64()
        .ok_or_else(|| PyValueError::new_err("expected unsigned DPT value"))?;

    u16::try_from(value).map_err(|_| PyValueError::new_err("DPT value is out of u16 range"))
}

fn json_i16(value: &Value) -> PyResult<i16> {
    let value = value
        .as_i64()
        .ok_or_else(|| PyValueError::new_err("expected signed DPT value"))?;

    i16::try_from(value).map_err(|_| PyValueError::new_err("DPT value is out of i16 range"))
}

fn json_u32(value: &Value) -> PyResult<u32> {
    let value = value
        .as_u64()
        .ok_or_else(|| PyValueError::new_err("expected unsigned DPT value"))?;

    u32::try_from(value).map_err(|_| PyValueError::new_err("DPT value is out of u32 range"))
}

fn json_i32(value: &Value) -> PyResult<i32> {
    let value = value
        .as_i64()
        .ok_or_else(|| PyValueError::new_err("expected signed DPT value"))?;

    i32::try_from(value).map_err(|_| PyValueError::new_err("DPT value is out of i32 range"))
}

// i64 (DPT29 V64) crosses JSON as a decimal string so values outside
// JavaScript's safe integer range are preserved exactly.
fn json_i64_str(value: &Value) -> PyResult<i64> {
    value
        .as_str()
        .ok_or_else(|| PyValueError::new_err("expected i64 decimal string"))?
        .parse::<i64>()
        .map_err(|_| PyValueError::new_err("DPT value is out of i64 range"))
}

// char (DPT4) crosses JSON as a string of exactly one character.
fn json_char(value: &Value) -> PyResult<char> {
    let text = value
        .as_str()
        .ok_or_else(|| PyValueError::new_err("expected a 1-character string"))?;
    let mut chars = text.chars();
    match (chars.next(), chars.next()) {
        (Some(c), None) => Ok(c),
        _ => Err(PyValueError::new_err(
            "expected exactly one character for a DPT4 value",
        )),
    }
}

fn json_f32(value: &Value) -> PyResult<f32> {
    value
        .as_f64()
        .map(|value| value as f32)
        .ok_or_else(|| PyValueError::new_err("expected numeric DPT value"))
}

fn json_string(value: &Value) -> PyResult<String> {
    value
        .as_str()
        .map(ToOwned::to_owned)
        .ok_or_else(|| PyValueError::new_err("expected string DPT value"))
}

fn to_py_value_error(error: impl std::fmt::Display) -> PyErr {
    PyValueError::new_err(error.to_string())
}

fn to_py_runtime_error(error: impl std::fmt::Display) -> PyErr {
    PyRuntimeError::new_err(error.to_string())
}

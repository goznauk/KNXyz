#![forbid(unsafe_code)]

use std::net::SocketAddr;
use std::str::FromStr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use knx_core::{GroupAddress, IndividualAddress};
use knx_dpt::DptValue;
use knx_ip::{DiscoveryOptions, TunnelClient, TunnelOptions};
use napi::bindgen_prelude::{Buffer, Result as NapiResult};
use napi_derive::napi;
use serde::Serialize;
use serde_json::Value;
use tokio::sync::Mutex;

#[napi(js_name = "encodeDptJson")]
pub fn encode_dpt_json(dpt: String, value_json: String) -> NapiResult<Buffer> {
    let value: Value = serde_json::from_str(&value_json).map_err(to_napi_error)?;
    let value = dpt_value_from_json(&dpt, &value)?;
    let bytes = knx_dpt::encode(&dpt, value).map_err(to_napi_error)?;

    Ok(bytes.into())
}

#[napi(js_name = "decodeDptJson")]
pub fn decode_dpt_json(dpt: String, bytes: Buffer) -> NapiResult<String> {
    let value = knx_dpt::decode(&dpt, bytes.as_ref()).map_err(to_napi_error)?;
    dpt_value_to_json(value)
}

#[napi(js_name = "parseIndividualAddress")]
pub fn parse_individual_address(value: String) -> NapiResult<String> {
    Ok(IndividualAddress::from_str(&value)
        .map_err(to_napi_error)?
        .to_string())
}

#[napi(js_name = "formatIndividualAddress")]
pub fn format_individual_address(value: String) -> NapiResult<String> {
    parse_individual_address(value)
}

#[napi(js_name = "parseGroupAddress")]
pub fn parse_group_address(value: String) -> NapiResult<String> {
    Ok(GroupAddress::from_str(&value)
        .map_err(to_napi_error)?
        .to_string())
}

#[napi(js_name = "formatGroupAddress")]
pub fn format_group_address(value: String) -> NapiResult<String> {
    parse_group_address(value)
}

#[napi(js_name = "discoverGatewaysJson")]
pub async fn discover_gateways_json(options_json: String) -> NapiResult<String> {
    let options = parse_discovery_options(&options_json)?;
    let gateways = knx_ip::discover_gateways(options)
        .await
        .map_err(to_napi_error)?;
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

    serde_json::to_string(&gateways).map_err(to_napi_error)
}

#[napi(js_name = "connectTunnelNative")]
pub async fn connect_tunnel_native(options_json: String) -> NapiResult<NativeTunnelClient> {
    let options: Value = serde_json::from_str(&options_json).map_err(to_napi_error)?;
    let options = parse_tunnel_options(&options)?;
    let client = TunnelClient::connect_with_options(options)
        .await
        .map_err(to_napi_error)?;

    Ok(NativeTunnelClient {
        inner: Mutex::new(client),
        closed: AtomicBool::new(false),
    })
}

#[napi(js_name = "NativeTunnelClient")]
pub struct NativeTunnelClient {
    inner: Mutex<TunnelClient>,
    closed: AtomicBool,
}

#[napi]
impl NativeTunnelClient {
    #[napi]
    pub async fn write(&self, group: String, dpt: String, value_json: String) -> NapiResult<()> {
        let group = GroupAddress::from_str(&group).map_err(to_napi_error)?;
        let value: Value = serde_json::from_str(&value_json).map_err(to_napi_error)?;
        let value = dpt_value_from_json(&dpt, &value)?;
        let mut client = self.inner.lock().await;

        client
            .group_write(group, value)
            .await
            .map_err(to_napi_error)
    }

    #[napi]
    pub async fn read(&self, group: String, dpt: String, timeout_ms: u32) -> NapiResult<String> {
        let group = GroupAddress::from_str(&group).map_err(to_napi_error)?;
        let mut client = self.inner.lock().await;
        let value = client
            .group_read(group, &dpt, Duration::from_millis(u64::from(timeout_ms)))
            .await
            .map_err(to_napi_error)?;

        dpt_value_to_json(value)
    }

    /// Best-effort orderly tunnel shutdown.
    ///
    /// Sends a real KNXnet/IP DISCONNECT_REQUEST through the underlying client
    /// and releases the connection. It is IDEMPOTENT (a second call is a no-op),
    /// it sends NO group write, and it never reconnects. The underlying
    /// `disconnect()` is itself ack-timeout-bounded, so it cannot hang on a
    /// silent/unreachable gateway; close then intentionally DISCARDS that result
    /// (best-effort), so it never throws on teardown. The TS layer also exposes
    /// this as `disconnect()`.
    #[napi]
    pub async fn close(&self) -> NapiResult<()> {
        if self.closed.swap(true, Ordering::SeqCst) {
            return Ok(());
        }
        let client = self.inner.lock().await;
        let _ = client.disconnect().await;
        Ok(())
    }
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

fn parse_discovery_options(input: &str) -> NapiResult<DiscoveryOptions> {
    let value: Value = serde_json::from_str(input).map_err(to_napi_error)?;
    let mut options = DiscoveryOptions::default();

    if let Some(bind) = value.get("bind").and_then(Value::as_str) {
        options.bind = bind.parse().map_err(to_napi_error)?;
    }
    if let Some(target) = value.get("target").and_then(Value::as_str) {
        options.target = target.parse().map_err(to_napi_error)?;
    }
    if let Some(timeout_ms) = value.get("timeoutMs").and_then(Value::as_u64) {
        options.timeout = Duration::from_millis(timeout_ms);
    }

    Ok(options)
}

fn tunnel_target_from_json(value: &Value) -> NapiResult<SocketAddr> {
    if let Some(target) = value.get("target").and_then(Value::as_str) {
        return target.parse().map_err(to_napi_error);
    }

    let host = value
        .get("host")
        .and_then(Value::as_str)
        .ok_or_else(|| napi::Error::from_reason("connectTunnel requires host or target"))?;
    let port = value.get("port").and_then(Value::as_u64).unwrap_or(3671);
    let port = u16::try_from(port)
        .map_err(|_| napi::Error::from_reason("connectTunnel port is out of range"))?;

    format!("{host}:{port}").parse().map_err(to_napi_error)
}

fn parse_tunnel_options(value: &Value) -> NapiResult<TunnelOptions> {
    let target = tunnel_target_from_json(value)?;
    let mut options = TunnelOptions::new(target);

    if let Some(bind) = value.get("bind").and_then(Value::as_str) {
        options.bind = bind.parse().map_err(to_napi_error)?;
    }
    if let Some(control_endpoint) = value.get("controlEndpoint").and_then(Value::as_str) {
        options.control_endpoint = Some(control_endpoint.parse().map_err(to_napi_error)?);
    }
    if let Some(data_endpoint) = value.get("dataEndpoint").and_then(Value::as_str) {
        options.data_endpoint = Some(data_endpoint.parse().map_err(to_napi_error)?);
    }
    if let Some(ack_timeout_ms) = value.get("ackTimeoutMs").and_then(Value::as_u64) {
        options.ack_timeout = Duration::from_millis(ack_timeout_ms);
    }

    Ok(options)
}

fn dpt_value_from_json(dpt: &str, value: &Value) -> NapiResult<DptValue> {
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
        // Untyped scene-number shorthand — parity with the Python binding
        // (bindings/python/src/lib.rs), so a BARE number under "17.001" is
        // accepted identically. `json_u8` enforces 0..=255; the unchanged
        // native dpt17 codec enforces the 0..=63 scene bound on encode. (The
        // tagged `{type:"scene_number",value}` form is already accepted above
        // via `dpt_value_from_typed_json`; this only adds the bare spelling and
        // opens no new capability — SceneNumber already maps to "17.001" in the
        // write path.)
        return json_u8(value).map(DptValue::SceneNumber);
    }

    Err(napi::Error::from_reason(format!(
        "unsupported DPT for Node binding: {dpt}"
    )))
}

fn dpt_value_from_typed_json(value: &Value) -> NapiResult<Option<DptValue>> {
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
        // i64 (DPT29 V64) crosses JSON as a DECIMAL STRING to survive JS number
        // precision; parse the string, never as_i64 on a (rounded) number.
        "i64" => DptValue::I64(json_i64_str(json_field(value, "value")?)?),
        // char (DPT4) crosses JSON as a 1-char string.
        "char" => DptValue::Char(json_char(json_field(value, "value")?)?),
        // DPT21/22 raw bitsets cross JSON as a bare u8/u16 number. from-json is
        // for round-trip/marshal only — encode still refuses (mains 21/22 absent
        // from the codec table; Bitset* loud-fails encode_value).
        "bitset8" => DptValue::Bitset8(json_u8(json_field(value, "value")?)?),
        "bitset16" => DptValue::Bitset16(json_u16(json_field(value, "value")?)?),
        _ => {
            return Err(napi::Error::from_reason(format!(
                "unsupported DPT JSON value type: {kind}"
            )));
        }
    };

    Ok(Some(value))
}

fn dpt_value_to_json(value: DptValue) -> NapiResult<String> {
    let value = match value {
        DptValue::Bool(value) => serde_json::json!(value),
        DptValue::U8(value) => serde_json::json!(value),
        // Float16 (weather 9.004/5/6/7) and Angle (5.003) decode to a bare
        // JSON number like Temperature/Scaling (no unit tag; decode-only).
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
        DptValue::SceneNumber(value) => {
            serde_json::json!({ "type": "scene_number", "value": value })
        }
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
        // i64 (DPT29 V64) is emitted as a DECIMAL STRING: its range exceeds the
        // JS safe-integer range (2^53), so a bare JSON number would be silently
        // corrupted when the JS side JSON.parses the decoded payload.
        DptValue::I64(value) => {
            serde_json::json!({ "type": "i64", "value": value.to_string() })
        }
        // char (DPT4) is emitted as a 1-char string.
        DptValue::Char(value) => {
            serde_json::json!({ "type": "char", "value": value.to_string() })
        }
        // DPT21/22 raw bitsets cross JSON as a bare number (u8/u16 fit the JS
        // safe-integer range, unlike i64 above which needs a decimal string).
        DptValue::Bitset8(value) => {
            serde_json::json!({ "type": "bitset8", "value": value })
        }
        DptValue::Bitset16(value) => {
            serde_json::json!({ "type": "bitset16", "value": value })
        }
    };

    serde_json::to_string(&value).map_err(to_napi_error)
}

fn json_field<'a>(value: &'a Value, field: &str) -> NapiResult<&'a Value> {
    value
        .get(field)
        .ok_or_else(|| napi::Error::from_reason(format!("expected DPT JSON field: {field}")))
}

fn json_bool(value: &Value) -> NapiResult<bool> {
    if let Some(value) = value.as_bool() {
        return Ok(value);
    }
    match value.as_u64() {
        Some(0) => Ok(false),
        Some(1) => Ok(true),
        _ => Err(napi::Error::from_reason("expected boolean DPT value")),
    }
}

fn json_i8(value: &Value) -> NapiResult<i8> {
    let value = value
        .as_i64()
        .ok_or_else(|| napi::Error::from_reason("expected signed DPT value"))?;

    i8::try_from(value).map_err(|_| napi::Error::from_reason("DPT value is out of i8 range"))
}

fn json_u8(value: &Value) -> NapiResult<u8> {
    let value = value
        .as_u64()
        .ok_or_else(|| napi::Error::from_reason("expected unsigned DPT value"))?;

    u8::try_from(value).map_err(|_| napi::Error::from_reason("DPT value is out of u8 range"))
}

fn json_u16(value: &Value) -> NapiResult<u16> {
    let value = value
        .as_u64()
        .ok_or_else(|| napi::Error::from_reason("expected unsigned DPT value"))?;

    u16::try_from(value).map_err(|_| napi::Error::from_reason("DPT value is out of u16 range"))
}

fn json_i16(value: &Value) -> NapiResult<i16> {
    let value = value
        .as_i64()
        .ok_or_else(|| napi::Error::from_reason("expected signed DPT value"))?;

    i16::try_from(value).map_err(|_| napi::Error::from_reason("DPT value is out of i16 range"))
}

fn json_u32(value: &Value) -> NapiResult<u32> {
    let value = value
        .as_u64()
        .ok_or_else(|| napi::Error::from_reason("expected unsigned DPT value"))?;

    u32::try_from(value).map_err(|_| napi::Error::from_reason("DPT value is out of u32 range"))
}

fn json_i32(value: &Value) -> NapiResult<i32> {
    let value = value
        .as_i64()
        .ok_or_else(|| napi::Error::from_reason("expected signed DPT value"))?;

    i32::try_from(value).map_err(|_| napi::Error::from_reason("DPT value is out of i32 range"))
}

// i64 (DPT29 V64) crosses JSON as a DECIMAL STRING (not a number): a bare i64
// number would already have lost precision passing through JS, so the contract
// is a string parsed here, never as_i64.
fn json_i64_str(value: &Value) -> NapiResult<i64> {
    value
        .as_str()
        .ok_or_else(|| napi::Error::from_reason("expected i64 decimal string"))?
        .parse::<i64>()
        .map_err(|_| napi::Error::from_reason("DPT value is out of i64 range"))
}

// char (DPT4) crosses JSON as a string of EXACTLY one character.
fn json_char(value: &Value) -> NapiResult<char> {
    let text = value
        .as_str()
        .ok_or_else(|| napi::Error::from_reason("expected a 1-character string"))?;
    let mut chars = text.chars();
    match (chars.next(), chars.next()) {
        (Some(c), None) => Ok(c),
        _ => Err(napi::Error::from_reason(
            "expected exactly one character for a DPT4 value",
        )),
    }
}

fn json_f32(value: &Value) -> NapiResult<f32> {
    value
        .as_f64()
        .map(|value| value as f32)
        .ok_or_else(|| napi::Error::from_reason("expected numeric DPT value"))
}

fn json_string(value: &Value) -> NapiResult<String> {
    value
        .as_str()
        .map(ToOwned::to_owned)
        .ok_or_else(|| napi::Error::from_reason("expected string DPT value"))
}

fn to_napi_error(error: impl std::fmt::Display) -> napi::Error {
    napi::Error::from_reason(error.to_string())
}

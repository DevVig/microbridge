//! Compact JSON-RPC helpers matching Work Louder / OAI device firmware.
//!
//! Requests are `{ method, params, id }` (no `jsonrpc: "2.0"` field).
//! IDs must stay in `0..999` per firmware limits in `wl-device-kit`.

use serde::Serialize;
use serde_json::{json, Value};

/// Per-thread accent lighting (Agent Key / thread LEDs).
pub const METHOD_THREADS_LIGHTING: &str = "v.oai.thstatus";

/// Keys + ambient ring lighting config.
pub const METHOD_RGB_CONFIG: &str = "v.oai.rgbcfg";

/// Device → host: custom HID key event.
pub const NOTIFY_HID: &str = "v.oai.hid";

/// Device → host: joystick / radial pad.
pub const NOTIFY_JOYSTICK: &str = "v.oai.rad";

/// Built-in LED animation effects (`OAILightingEffect`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum LightingEffect {
    Off = 0,
    Solid = 1,
    Snake = 2,
    Rainbow = 3,
    Breath = 4,
    Gradient = 5,
    ShallowBreath = 6,
}

/// Minimized per-thread lighting entry (`sendThreadsLighting`).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ThreadLightingParam {
    pub id: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub c: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub b: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub e: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub s: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sk: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sa: Option<u8>,
}

/// Build a JSON-RPC request body (UTF-8). `id` is clamped to `0..999`.
pub fn build_request(method: &str, params: Value, id: u32) -> String {
    let id = id % 999;
    serde_json::to_string(&json!({
        "method": method,
        "params": params,
        "id": id,
    }))
    .expect("json request serialization")
}

/// Build a `v.oai.thstatus` request from minimized thread entries.
pub fn threads_lighting_request(threads: &[ThreadLightingParam], id: u32) -> String {
    let params = serde_json::to_value(threads).expect("thread params");
    build_request(METHOD_THREADS_LIGHTING, params, id)
}

/// Parsed device → host notification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeviceNotify {
    Hid {
        key: String,
        act: Option<i64>,
        agent: Option<i64>,
    },
    Joystick {
        angle: Option<i64>,
        distance: Option<i64>,
    },
    Other {
        method: String,
    },
}

/// Parse a complete JSON notification / response line from the device.
pub fn parse_notify(line: &str) -> Option<DeviceNotify> {
    let value: Value = serde_json::from_str(line.trim()).ok()?;
    // Responses have `id`; notifications have `method`/`m` only.
    if value.get("id").is_some() || value.get("i").is_some() {
        return None;
    }
    let method = value
        .get("method")
        .or_else(|| value.get("m"))
        .and_then(|v| v.as_str())?;
    let params = value.get("params").cloned().unwrap_or(Value::Null);
    match method {
        NOTIFY_HID => {
            let key = params.get("k")?.as_str()?.to_string();
            Some(DeviceNotify::Hid {
                key,
                act: params.get("act").and_then(|v| v.as_i64()),
                agent: params.get("ag").and_then(|v| v.as_i64()),
            })
        }
        NOTIFY_JOYSTICK => Some(DeviceNotify::Joystick {
            angle: params.get("a").and_then(|v| v.as_i64()),
            distance: params.get("d").and_then(|v| v.as_i64()),
        }),
        other => Some(DeviceNotify::Other {
            method: other.to_string(),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn threads_request_shape() {
        let req = threads_lighting_request(
            &[ThreadLightingParam {
                id: 0,
                c: Some(0x3D7EFF),
                b: Some(0.8),
                e: Some(LightingEffect::Solid as u8),
                s: None,
                sk: Some(1),
                sa: None,
            }],
            42,
        );
        let v: Value = serde_json::from_str(&req).unwrap();
        assert_eq!(v["method"], METHOD_THREADS_LIGHTING);
        assert_eq!(v["id"], 42);
        assert_eq!(v["params"][0]["id"], 0);
        assert_eq!(v["params"][0]["c"], 0x3D7EFF);
        assert!(v.get("jsonrpc").is_none());
    }

    #[test]
    fn parses_hid_notify() {
        let n = parse_notify(r#"{"method":"v.oai.hid","params":{"k":"agent1","act":1,"ag":0}}"#)
            .unwrap();
        assert_eq!(
            n,
            DeviceNotify::Hid {
                key: "agent1".into(),
                act: Some(1),
                agent: Some(0),
            }
        );
    }
}

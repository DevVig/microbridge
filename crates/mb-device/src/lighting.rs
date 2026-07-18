//! Map Microbridge [`LedFrame`] values onto Work Louder thread-lighting RPC.

use mb_protocol::AgentState;

use crate::rpc::{threads_lighting_request, LightingEffect, ThreadLightingParam};
use crate::LedFrame;

/// Convert a daemon LED frame into a `v.oai.thstatus` JSON-RPC request.
pub fn threads_lighting_rpc(frame: &LedFrame, rpc_id: u32) -> String {
    let threads = frame_to_threads(frame);
    threads_lighting_request(&threads, rpc_id)
}

fn frame_to_threads(frame: &LedFrame) -> Vec<ThreadLightingParam> {
    let brightness = if frame.paused {
        0.0
    } else {
        (frame.brightness as f64 / 100.0).clamp(0.0, 1.0)
    };

    frame
        .keys
        .iter()
        .enumerate()
        .map(|(id, state)| {
            let focused = frame.focus_index == Some(id);
            match (frame.paused, state, frame.key_colors[id]) {
                (true, _, _) | (_, None, _) => ThreadLightingParam {
                    id: id as u32,
                    c: None,
                    b: Some(0.0),
                    e: Some(LightingEffect::Off as u8),
                    s: None,
                    sk: None,
                    sa: None,
                },
                (_, Some(agent_state), color) => ThreadLightingParam {
                    id: id as u32,
                    c: color.or_else(|| Some(fallback_color(*agent_state))),
                    b: Some(brightness),
                    e: Some(effect_for(*agent_state) as u8),
                    s: speed_for(*agent_state),
                    sk: if focused { Some(1) } else { None },
                    sa: if focused { Some(1) } else { None },
                },
            }
        })
        .collect()
}

fn effect_for(state: AgentState) -> LightingEffect {
    match state {
        AgentState::Idle | AgentState::Done => LightingEffect::Solid,
        AgentState::Thinking => LightingEffect::ShallowBreath,
        AgentState::Working => LightingEffect::Solid,
        AgentState::AwaitingApproval => LightingEffect::Breath,
        AgentState::Error => LightingEffect::Solid,
    }
}

fn speed_for(state: AgentState) -> Option<f64> {
    match state {
        AgentState::Thinking | AgentState::AwaitingApproval => Some(0.55),
        _ => None,
    }
}

fn fallback_color(state: AgentState) -> u32 {
    // Codex preset defaults from `StateColors::codex` when the daemon omits RGB.
    match state {
        AgentState::Idle => 0xE9E9E6,
        AgentState::Thinking | AgentState::Working => 0x3D7EFF,
        AgentState::AwaitingApproval => 0xFFB000,
        AgentState::Done => 0x30C463,
        AgentState::Error => 0xFF453A,
    }
}

/// Parse `#RRGGBB` / `RRGGBB` into a packed RGB integer for the device.
pub fn parse_rgb_hex(s: &str) -> Option<u32> {
    let hex = s.trim().trim_start_matches('#');
    if hex.len() != 6 {
        return None;
    }
    u32::from_str_radix(hex, 16).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use mb_protocol::AgentState;
    use serde_json::Value;

    #[test]
    fn packs_six_threads() {
        let mut frame = LedFrame {
            brightness: 80,
            ..LedFrame::default()
        };
        frame.keys[0] = Some(AgentState::Working);
        frame.keys[1] = Some(AgentState::AwaitingApproval);
        frame.key_colors[0] = Some(0x112233);
        frame.focus_index = Some(0);

        let req = threads_lighting_rpc(&frame, 7);
        let v: Value = serde_json::from_str(&req).unwrap();
        assert_eq!(v["params"].as_array().unwrap().len(), 6);
        assert_eq!(v["params"][0]["c"], 0x112233);
        assert_eq!(v["params"][0]["sk"], 1);
        assert_eq!(v["params"][1]["e"], LightingEffect::Breath as u8);
        assert_eq!(v["params"][2]["e"], LightingEffect::Off as u8);
    }

    #[test]
    fn parse_rgb_hex_works() {
        assert_eq!(parse_rgb_hex("#3D7EFF"), Some(0x3D7EFF));
        assert_eq!(parse_rgb_hex("ff453a"), Some(0xFF453A));
    }
}

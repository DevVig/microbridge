//! Device abstraction: turns resolved agent state into hardware output.
//!
//! Real Codex Micro HID support lands behind [`HidDevice`] (best-effort).
//! Until a device is present the daemon drives [`MockDevice`], which logs the
//! frames a real device would render. Protocol constants and packing live in
//! this crate — see `docs/device-hid.md`.

mod framing;
mod ids;
mod lighting;
mod probe;
mod rpc;

#[cfg(feature = "hid")]
mod capture;
#[cfg(feature = "hid")]
mod claim;

#[cfg(feature = "hid")]
pub use capture::run_capture;

pub use framing::{frame_rpc, parse_report, CHANNEL_DEBUG, CHANNEL_RPC, REPORT_ID};
pub use ids::{is_supported_pid, CODEX_MICRO_PID, WL_MANUFACTURERS, WL_USAGE_PAGE, WL_VID};
pub use lighting::{parse_rgb_hex, threads_lighting_rpc};
pub use probe::{match_usb_text, probe_usb_micro, ProbeResult};
pub use rpc::{
    parse_notify, threads_lighting_request, DeviceNotify, LightingEffect, METHOD_RGB_CONFIG,
    METHOD_THREADS_LIGHTING,
};

use mb_protocol::{AgentState, AGENT_KEY_COUNT};

/// Descriptor-driven layout reported by a connected device (or the mock).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceDescriptor {
    pub name: String,
    pub agent_key_count: usize,
    pub has_dial: bool,
    pub has_joystick: bool,
    pub connected: bool,
}

impl Default for DeviceDescriptor {
    fn default() -> Self {
        Self {
            name: "mock".into(),
            agent_key_count: AGENT_KEY_COUNT,
            has_dial: true,
            has_joystick: true,
            connected: false,
        }
    }
}

/// Physical / logical input from the deck.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeviceInput {
    /// Agent Key index 0..5 — single press.
    AgentKeyPress {
        index: usize,
    },
    /// Agent Key index 0..5 — second press within 350ms.
    AgentKeyDoublePress {
        index: usize,
    },
    Approve,
    Reject,
    Interrupt,
    NewSession,
    CycleFocus,
    DialRotate {
        delta: i8,
    },
    DialPress,
    JoystickFlick {
        direction: JoystickDir,
    },
    TouchTap,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JoystickDir {
    Up,
    Down,
    Left,
    Right,
}

/// Frame rendered onto the six Agent Keys (+ optional focus highlight).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LedFrame {
    pub keys: [Option<AgentState>; AGENT_KEY_COUNT],
    /// Packed RGB (`0xRRGGBB`) per key when the daemon has resolved palette colors.
    pub key_colors: [Option<u32>; AGENT_KEY_COUNT],
    pub focus_index: Option<usize>,
    pub brightness: u8,
    pub paused: bool,
}

impl Default for LedFrame {
    fn default() -> Self {
        Self {
            keys: [None; AGENT_KEY_COUNT],
            key_colors: [None; AGENT_KEY_COUNT],
            focus_index: None,
            brightness: 80,
            paused: false,
        }
    }
}

pub trait Device: Send {
    fn descriptor(&self) -> DeviceDescriptor;

    /// Render Agent Key LEDs. Called only on transitions — implementations
    /// may assume calls are rare and need not debounce.
    fn set_leds(&mut self, frame: &LedFrame);

    /// Poll for a pending input event (non-blocking). Mock returns None.
    fn poll_input(&mut self) -> Option<DeviceInput> {
        None
    }
}

/// Logs what a real device would display.
#[derive(Debug, Default)]
pub struct MockDevice {
    last: LedFrame,
}

impl Device for MockDevice {
    fn descriptor(&self) -> DeviceDescriptor {
        DeviceDescriptor {
            name: "mock".into(),
            agent_key_count: AGENT_KEY_COUNT,
            has_dial: true,
            has_joystick: true,
            // Not a physical device — UI treats `device_name == "mock"` as simulator.
            connected: false,
        }
    }

    fn set_leds(&mut self, frame: &LedFrame) {
        if &self.last != frame {
            tracing::info!(
                device = "mock",
                keys = ?frame.keys,
                focus = ?frame.focus_index,
                paused = frame.paused,
                "render frame"
            );
            self.last = frame.clone();
        }
    }
}

/// Best-effort USB HID driver for the Codex Micro.
///
/// Without a probed device this behaves like [`MockDevice`] and reports
/// `connected: false`. Presence (Detected) uses VID/PID from ChatGPT's kit.
/// Live claim requires `--features hid` and `MICROBRIDGE_HID_CLAIM=1`.
pub struct HidDevice {
    inner: MockDevice,
    /// True only when the vendor HID interface is claimed for writes.
    connected: bool,
    /// USB present (Detected) even if not claimed.
    usb_present: bool,
    name: String,
    product_id: Option<u16>,
    rpc_seq: u32,
    #[cfg(feature = "hid")]
    claimed: Option<claim::ClaimedDevice>,
}

impl std::fmt::Debug for HidDevice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HidDevice")
            .field("connected", &self.connected)
            .field("usb_present", &self.usb_present)
            .field("name", &self.name)
            .field("product_id", &self.product_id)
            .finish_non_exhaustive()
    }
}

impl Default for HidDevice {
    fn default() -> Self {
        Self {
            inner: MockDevice::default(),
            connected: false,
            usb_present: false,
            name: "codex-micro".into(),
            product_id: None,
            rpc_seq: 1,
            #[cfg(feature = "hid")]
            claimed: None,
        }
    }
}

impl HidDevice {
    /// Attempt to open the first matching USB device. Falls back to
    /// disconnected (mock rendering) when none is found or HID is unavailable.
    ///
    /// Claim is opt-in (`MICROBRIDGE_HID_CLAIM=1` + `hid` feature) so we do not
    /// fight ChatGPT Desktop by default.
    pub fn open() -> Self {
        let probe = probe_usb_micro();
        if !probe.present {
            return Self::default();
        }

        let pid = probe.product_id.unwrap_or(CODEX_MICRO_PID);
        let name = format!("{}-usb", ids::product_name(pid));
        let mut device = Self {
            inner: MockDevice::default(),
            connected: false,
            usb_present: true,
            name,
            product_id: Some(pid),
            rpc_seq: 1,
            #[cfg(feature = "hid")]
            claimed: None,
        };

        if claim_requested() {
            device.try_claim();
        }
        device
    }

    pub fn set_connected_for_tests(&mut self, connected: bool) {
        self.connected = connected;
        self.usb_present = connected || self.usb_present;
    }

    pub fn usb_present(&self) -> bool {
        self.usb_present
    }

    #[cfg(feature = "hid")]
    fn try_claim(&mut self) {
        match claim::open_device(self.product_id) {
            Ok(claimed) => {
                tracing::info!(
                    product_id = format_args!("0x{:04X}", claimed.product_id),
                    name = %claimed.name,
                    "claimed Work Louder HID interface"
                );
                self.name = claimed.name.clone();
                self.product_id = Some(claimed.product_id);
                self.connected = true;
                self.claimed = Some(claimed);
            }
            Err(error) => {
                tracing::warn!(%error, "HID claim requested but open failed; staying Detected-only");
            }
        }
    }

    #[cfg(not(feature = "hid"))]
    fn try_claim(&mut self) {
        tracing::warn!(
            "MICROBRIDGE_HID_CLAIM set but mb-device built without `hid` feature — Detected only"
        );
    }

    fn next_rpc_id(&mut self) -> u32 {
        #[cfg(feature = "hid")]
        if let Some(claimed) = self.claimed.as_mut() {
            return claimed.next_rpc_id();
        }
        let id = self.rpc_seq;
        self.rpc_seq = (self.rpc_seq % 998) + 1;
        id
    }
}

fn claim_requested() -> bool {
    matches!(
        std::env::var("MICROBRIDGE_HID_CLAIM").as_deref(),
        Ok("1") | Ok("true") | Ok("TRUE") | Ok("yes") | Ok("YES")
    )
}

impl Device for HidDevice {
    fn descriptor(&self) -> DeviceDescriptor {
        DeviceDescriptor {
            name: self.name.clone(),
            agent_key_count: AGENT_KEY_COUNT,
            has_dial: true,
            has_joystick: true,
            connected: self.connected,
        }
    }

    fn set_leds(&mut self, frame: &LedFrame) {
        let rpc_id = self.next_rpc_id();
        let request = threads_lighting_rpc(frame, rpc_id);
        let reports = frame_rpc(&request);

        if self.connected {
            #[cfg(feature = "hid")]
            if let Some(claimed) = self.claimed.as_ref() {
                if let Err(error) = claimed.write_rpc(&request) {
                    tracing::warn!(%error, "failed to write thread lighting RPC");
                } else {
                    tracing::debug!(
                        device = %self.name,
                        bytes = request.len(),
                        packets = reports.len(),
                        "hid rpc v.oai.thstatus"
                    );
                }
            }
            #[cfg(not(feature = "hid"))]
            {
                let _ = reports;
                tracing::debug!(device = %self.name, %request, "hid led rpc (no claim backend)");
            }
        } else if self.usb_present {
            tracing::debug!(
                device = %self.name,
                packets = reports.len(),
                request = %request,
                "usb present; packed LED RPC (set MICROBRIDGE_HID_CLAIM=1 to write)"
            );
        }

        self.inner.set_leds(frame);
    }

    fn poll_input(&mut self) -> Option<DeviceInput> {
        #[cfg(feature = "hid")]
        if let Some(claimed) = self.claimed.as_mut() {
            for notify in claimed.poll_notifies() {
                if let Some(input) = notify_to_input(notify) {
                    return Some(input);
                }
            }
        }
        None
    }
}

#[cfg(feature = "hid")]
fn notify_to_input(notify: DeviceNotify) -> Option<DeviceInput> {
    match notify {
        DeviceNotify::Hid { key, .. } => {
            agent_key_index(&key).map(|index| DeviceInput::AgentKeyPress { index })
        }
        DeviceNotify::Joystick { angle, .. } => angle.and_then(joystick_from_angle),
        DeviceNotify::Other { .. } => None,
    }
}

fn agent_key_index(key: &str) -> Option<usize> {
    // Firmware may use agent0..agent5, agent1..agent6, or bare digits — accept common forms.
    let digits: String = key.chars().filter(|c| c.is_ascii_digit()).collect();
    if digits.is_empty() {
        return None;
    }
    let n: usize = digits.parse().ok()?;
    if (1..=AGENT_KEY_COUNT).contains(&n) {
        Some(n - 1)
    } else if n < AGENT_KEY_COUNT {
        Some(n)
    } else {
        None
    }
}

fn joystick_from_angle(angle: i64) -> Option<DeviceInput> {
    // Degrees → cardinal flick; exact firmware mapping validated on hardware.
    let a = angle.rem_euclid(360);
    let direction = match a {
        45..=134 => JoystickDir::Right,
        135..=224 => JoystickDir::Down,
        225..=314 => JoystickDir::Left,
        _ => JoystickDir::Up,
    };
    Some(DeviceInput::JoystickFlick { direction })
}

/// Prefer a claimed HID device; else a detected-but-unclaimed USB Micro;
/// else the mock simulator.
pub fn open_default_device() -> Box<dyn Device> {
    let hid = HidDevice::open();
    if hid.usb_present() || hid.descriptor().connected {
        Box::new(hid)
    } else {
        Box::new(MockDevice::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Characterization tests: these lock in the *current guessed* mapping so a
    // regression is visible. The real `v.oai.hid` strings get confirmed with a
    // physical unit via `microbridgectl hid-capture` — see
    // docs/hardware-bringup.md. Update these alongside the real map.

    #[test]
    fn agent_key_index_prefers_one_based_forms() {
        // ChatGPT ships `agent1..agent6`; treat 1..=6 as one-based (0..=5).
        assert_eq!(agent_key_index("agent1"), Some(0));
        assert_eq!(agent_key_index("agent6"), Some(5));
        assert_eq!(agent_key_index("k3"), Some(2));
    }

    #[test]
    fn agent_key_index_accepts_zero_based_zero() {
        // A bare 0 can only mean the first key.
        assert_eq!(agent_key_index("agent0"), Some(0));
        assert_eq!(agent_key_index("0"), Some(0));
    }

    #[test]
    fn agent_key_index_rejects_out_of_range_and_digitless() {
        assert_eq!(agent_key_index("agent7"), None);
        assert_eq!(agent_key_index("approve"), None);
        assert_eq!(agent_key_index(""), None);
    }

    #[test]
    fn joystick_angle_maps_to_cardinals() {
        let dir = |a| match joystick_from_angle(a) {
            Some(DeviceInput::JoystickFlick { direction }) => direction,
            other => panic!("expected a flick, got {other:?}"),
        };
        assert_eq!(dir(0), JoystickDir::Up);
        assert_eq!(dir(90), JoystickDir::Right);
        assert_eq!(dir(180), JoystickDir::Down);
        assert_eq!(dir(270), JoystickDir::Left);
        // Wraps: 360 ≡ 0, and negatives normalize via rem_euclid.
        assert_eq!(dir(360), JoystickDir::Up);
        assert_eq!(dir(-90), JoystickDir::Left);
    }
}

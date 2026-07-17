//! Device abstraction: turns resolved agent state into hardware output.
//!
//! Real Codex Micro HID support lands behind [`HidDevice`] (best-effort).
//! Until a device is present the daemon drives [`MockDevice`], which logs the
//! frames a real device would render. All reverse-engineering stays in this
//! crate — see `docs/device-hid.md`.

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
    pub focus_index: Option<usize>,
    pub brightness: u8,
    pub paused: bool,
}

impl Default for LedFrame {
    fn default() -> Self {
        Self {
            keys: [None; AGENT_KEY_COUNT],
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
            // Simulator is "attached" so the menu bar UI is usable without HID.
            // Real HidDevice reports connected only when a USB device is claimed.
            connected: true,
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
/// `connected: false`. Real report packing is documented in
/// `docs/device-hid.md` and filled in as the HID map is confirmed.
#[derive(Debug)]
pub struct HidDevice {
    inner: MockDevice,
    connected: bool,
    name: String,
}

impl Default for HidDevice {
    fn default() -> Self {
        Self {
            inner: MockDevice::default(),
            connected: false,
            name: "codex-micro".into(),
        }
    }
}

impl HidDevice {
    /// Attempt to open the first matching USB device. Falls back to
    /// disconnected (mock rendering) when none is found or HID is unavailable.
    pub fn open() -> Self {
        // HID open is gated on platform + discovered VID/PID (see docs).
        // Until the report map is verified we never claim exclusive access.
        Self::default()
    }

    pub fn set_connected_for_tests(&mut self, connected: bool) {
        self.connected = connected;
    }
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
        if self.connected {
            tracing::debug!(device = %self.name, keys = ?frame.keys, "hid led frame");
        }
        self.inner.set_leds(frame);
    }
}

/// Prefer a live HID device when one can be claimed; otherwise mock.
pub fn open_default_device() -> Box<dyn Device> {
    let hid = HidDevice::open();
    if hid.descriptor().connected {
        Box::new(hid)
    } else {
        Box::new(MockDevice::default())
    }
}

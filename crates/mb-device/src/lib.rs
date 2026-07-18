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
    ///
    /// Until the report map is verified we never claim exclusive access. On
    /// macOS we still probe USB presence so the UI can show "Detected".
    pub fn open() -> Self {
        if usb_micro_present() {
            Self {
                inner: MockDevice::default(),
                connected: false,
                name: "codex-micro-usb".into(),
            }
        } else {
            Self::default()
        }
    }

    pub fn set_connected_for_tests(&mut self, connected: bool) {
        self.connected = connected;
    }
}

/// Best-effort USB presence probe — does not claim the interface.
fn usb_micro_present() -> bool {
    #[cfg(target_os = "macos")]
    {
        match system_profiler_usb_text(std::time::Duration::from_secs(3)) {
            Some(text) => usb_text_matches_micro(&text),
            None => false,
        }
    }
    #[cfg(not(target_os = "macos"))]
    {
        false
    }
}

#[cfg(target_os = "macos")]
fn system_profiler_usb_text(timeout: std::time::Duration) -> Option<String> {
    use std::io::Read;
    use std::process::{Command, Stdio};
    use std::thread;
    use std::time::Instant;

    let mut child = Command::new("system_profiler")
        .args(["SPUSBDataType", "-detailLevel", "mini"])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    let deadline = Instant::now() + timeout;
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                if !status.success() {
                    return None;
                }
                let mut buf = Vec::new();
                if let Some(mut out) = child.stdout.take() {
                    let _ = out.read_to_end(&mut buf);
                }
                return Some(String::from_utf8_lossy(&buf).into_owned());
            }
            Ok(None) if Instant::now() >= deadline => {
                let _ = child.kill();
                let _ = child.wait();
                return None;
            }
            Ok(None) => thread::sleep(std::time::Duration::from_millis(50)),
            Err(_) => return None,
        }
    }
}

#[cfg(target_os = "macos")]
fn usb_text_matches_micro(raw: &str) -> bool {
    let text = raw.to_ascii_lowercase();
    if text.contains("codex micro") {
        return true;
    }
    // Require manufacturer + product token in the same USB record block.
    text.split("\n\n").any(|block| {
        block.contains("work louder") && (block.contains("codex") || block.contains("kbd-1.0"))
    })
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

/// Prefer a claimed HID device; else a detected-but-unclaimed USB Micro;
/// else the mock simulator.
pub fn open_default_device() -> Box<dyn Device> {
    let hid = HidDevice::open();
    let desc = hid.descriptor();
    if desc.connected || desc.name == "codex-micro-usb" {
        Box::new(hid)
    } else {
        Box::new(MockDevice::default())
    }
}

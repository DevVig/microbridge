//! Optional HID claim via `hidapi`.
//!
//! Enabled with `--features hid`. Claim is still opt-in at runtime via
//! `MICROBRIDGE_HID_CLAIM=1` so ChatGPT Desktop can keep the device by default.

#![cfg(feature = "hid")]

use std::sync::Mutex;

use hidapi::{HidApi, HidDevice as RawHid};

use crate::framing::{frame_rpc, parse_report, CHANNEL_RPC, REPORT_ID};
use crate::ids::{is_supported_pid, product_name, WL_USAGE_PAGE, WL_VID};
use crate::probe::{hid_candidate_sort_key, transport_from_bus_type};
use crate::rpc::{parse_notify, DeviceNotify};
use crate::DeviceTransport;

/// Open the first matching vendor HID interface (usage page `0xFF00`).
///
/// On macOS, opens non-exclusively so ChatGPT Desktop can coexist.
pub fn open_device(preferred_pid: Option<u16>) -> Result<ClaimedDevice, String> {
    let api = HidApi::new().map_err(|e| e.to_string())?;

    #[cfg(target_os = "macos")]
    {
        // Match ChatGPT Desktop / node-hid `nonExclusive: true`.
        api.set_open_exclusive(false);
    }

    let mut candidates: Vec<_> = api
        .device_list()
        .filter(|info| {
            info.vendor_id() == WL_VID
                && is_supported_pid(info.product_id())
                && info.usage_page() == WL_USAGE_PAGE
        })
        .collect();

    candidates.sort_by_key(|info| hid_candidate_sort_key(info, preferred_pid));

    let info = candidates
        .first()
        .ok_or_else(|| "no Work Louder vendor HID interface found".to_string())?;

    let product_id = info.product_id();
    let name = product_name(product_id).to_string();
    let transport = transport_from_bus_type(info.bus_type());
    let device = api
        .open_path(info.path())
        .map_err(|e| format!("open_path failed: {e}"))?;
    let _ = device.set_blocking_mode(false);

    Ok(ClaimedDevice {
        device: Mutex::new(device),
        product_id,
        name,
        transport,
        rpc_id: 1,
        rx_buf: String::new(),
        pending: Vec::new(),
    })
}

/// A claimed vendor HID channel that can write RPC and poll notifications.
pub struct ClaimedDevice {
    device: Mutex<RawHid>,
    pub product_id: u16,
    pub name: String,
    pub transport: DeviceTransport,
    rpc_id: u32,
    rx_buf: String,
    pending: Vec<DeviceNotify>,
}

impl ClaimedDevice {
    pub fn next_rpc_id(&mut self) -> u32 {
        let id = self.rpc_id;
        self.rpc_id = (self.rpc_id + 1) % 999;
        if self.rpc_id == 0 {
            self.rpc_id = 1;
        }
        id
    }

    /// Write a JSON-RPC request string (already serialized).
    pub fn write_rpc(&self, request: &str) -> Result<(), String> {
        let reports = frame_rpc(request);
        let dev = self.device.lock().map_err(|e| e.to_string())?;
        for report in reports {
            // hidapi expects the report id in byte 0 for write().
            debug_assert_eq!(report[0], REPORT_ID);
            dev.write(&report)
                .map_err(|e| format!("hid write failed: {e}"))?;
        }
        Ok(())
    }

    /// Non-blocking read; accumulates RPC channel text and parses notifies.
    pub fn poll_notifies(&mut self) -> Vec<DeviceNotify> {
        {
            let dev = match self.device.lock() {
                Ok(d) => d,
                Err(_) => return Vec::new(),
            };
            let mut buf = [0u8; 64];
            loop {
                match dev.read_timeout(&mut buf, 0) {
                    Ok(n) if n > 0 => {
                        if let Some(packet) = parse_report(&buf[..n]) {
                            if packet.channel == CHANNEL_RPC {
                                if let Ok(text) = std::str::from_utf8(&packet.payload) {
                                    self.rx_buf.push_str(text);
                                }
                            }
                        }
                    }
                    _ => break,
                }
            }
        }

        let mut out = std::mem::take(&mut self.pending);
        while let Some(idx) = self.rx_buf.find('\n') {
            let line = self.rx_buf[..idx].trim_end_matches('\r').to_string();
            self.rx_buf = self.rx_buf[idx + 1..].to_string();
            if let Some(n) = parse_notify(&line) {
                out.push(n);
            }
        }
        // Also try parse if buffer looks like a complete JSON object w/o newline yet.
        if self.rx_buf.trim_start().starts_with('{') {
            if let Some(n) = parse_notify(&self.rx_buf) {
                out.push(n);
                self.rx_buf.clear();
            }
        }
        out
    }
}

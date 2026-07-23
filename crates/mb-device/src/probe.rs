//! Best-effort HID presence probe — does not claim the HID interface.

use crate::ids::{is_supported_pid, CODEX_MICRO_PID, WL_VID};

/// Result of a non-claiming HID-registry probe, with a USB fallback.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProbeResult {
    pub present: bool,
    /// Best-effort product id reported by the HID registry or host USB listing.
    pub product_id: Option<u16>,
    pub transport: DeviceTransport,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum DeviceTransport {
    Usb,
    Bluetooth,
    #[default]
    Unknown,
}

impl DeviceTransport {
    pub fn suffix(self) -> &'static str {
        match self {
            Self::Usb => "usb",
            Self::Bluetooth => "bluetooth",
            Self::Unknown => "hid",
        }
    }

    fn preference_rank(self) -> u8 {
        match self {
            // A cable is the least ambiguous route when firmware exposes both
            // transports at once. Bluetooth remains preferred over an unknown
            // backend, and is selected whenever USB is absent.
            Self::Usb => 0,
            Self::Bluetooth => 1,
            Self::Unknown => 2,
        }
    }
}

#[cfg(feature = "hid")]
pub(crate) fn transport_from_bus_type(bus_type: hidapi::BusType) -> DeviceTransport {
    match bus_type {
        hidapi::BusType::Usb => DeviceTransport::Usb,
        hidapi::BusType::Bluetooth => DeviceTransport::Bluetooth,
        _ => DeviceTransport::Unknown,
    }
}

#[cfg(feature = "hid")]
pub(crate) fn hid_candidate_sort_key(
    info: &hidapi::DeviceInfo,
    preferred_pid: Option<u16>,
) -> (bool, u8, u16) {
    candidate_preference_key(
        info.product_id(),
        transport_from_bus_type(info.bus_type()),
        preferred_pid,
    )
}

fn candidate_preference_key(
    product_id: u16,
    transport: DeviceTransport,
    preferred_pid: Option<u16>,
) -> (bool, u8, u16) {
    (
        preferred_pid.is_some_and(|pid| product_id != pid),
        transport.preference_rank(),
        product_id,
    )
}

impl ProbeResult {
    pub fn absent() -> Self {
        Self {
            present: false,
            product_id: None,
            transport: DeviceTransport::Unknown,
        }
    }
}

/// Probe for a supported Work Louder / Codex Micro HID device.
///
/// The HID registry is authoritative because macOS exposes both USB and BLE
/// HID devices there. `system_profiler` remains a USB-only fallback for builds
/// compiled without the optional HID backend or if HID initialization fails.
pub fn probe_usb_micro() -> ProbeResult {
    #[cfg(feature = "hid")]
    match probe_hid_micro() {
        Ok(Some(result)) => return result,
        Ok(None) => return ProbeResult::absent(),
        Err(()) => {}
    }

    #[cfg(target_os = "macos")]
    {
        match system_profiler_usb_text(std::time::Duration::from_secs(3)) {
            Some(text) => {
                let mut result = match_usb_text(&text);
                if result.present {
                    result.transport = DeviceTransport::Usb;
                }
                result
            }
            None => ProbeResult::absent(),
        }
    }
    #[cfg(not(target_os = "macos"))]
    {
        ProbeResult::absent()
    }
}

#[cfg(feature = "hid")]
fn probe_hid_micro() -> Result<Option<ProbeResult>, ()> {
    use hidapi::HidApi;

    let api = HidApi::new().map_err(|_| ())?;
    let mut candidates: Vec<_> = api
        .device_list()
        .filter(|info| {
            info.vendor_id() == WL_VID
                && is_supported_pid(info.product_id())
                && info.usage_page() == crate::WL_USAGE_PAGE
        })
        .collect();
    candidates.sort_by_key(|info| hid_candidate_sort_key(info, None));
    let Some(info) = candidates.first() else {
        return Ok(None);
    };
    Ok(Some(ProbeResult {
        present: true,
        product_id: Some(info.product_id()),
        transport: transport_from_bus_type(info.bus_type()),
    }))
}

/// Match `system_profiler SPUSBDataType` (or similar) text against known IDs.
pub fn match_usb_text(raw: &str) -> ProbeResult {
    let lower = raw.to_ascii_lowercase();

    // Prefer explicit VID/PID pairs in the same USB record block.
    for block in lower.split("\n\n") {
        if let Some(pid) = block_matching_pid(block) {
            return ProbeResult {
                present: true,
                product_id: Some(pid),
                transport: DeviceTransport::Usb,
            };
        }
    }

    // Fallback: product name tokens (pre-VID listings / BT advertising names).
    if lower.contains("codex micro")
        || lower.split("\n\n").any(|block| {
            block.contains("work louder") && (block.contains("codex") || block.contains("kbd-1.0"))
        })
    {
        return ProbeResult {
            present: true,
            product_id: Some(CODEX_MICRO_PID),
            transport: DeviceTransport::Usb,
        };
    }

    ProbeResult::absent()
}

fn block_matching_pid(block: &str) -> Option<u16> {
    let vid = parse_id_field(block, "vendor id")?;
    if vid != WL_VID {
        return None;
    }
    let pid = parse_id_field(block, "product id")?;
    is_supported_pid(pid).then_some(pid)
}

fn parse_id_field(block: &str, label: &str) -> Option<u16> {
    let prefix = format!("{label}:");
    for line in block.lines() {
        let ll = line.trim().to_ascii_lowercase();
        if let Some(rest) = ll.strip_prefix(&prefix) {
            return parse_hex_id(rest.trim());
        }
    }
    None
}

fn parse_hex_id(raw: &str) -> Option<u16> {
    // Formats: "0x8360", "0x8360 (codex micro)", "8360"
    let token = raw.split_whitespace().next()?.trim();
    let hex = token.strip_prefix("0x").unwrap_or(token);
    u16::from_str_radix(hex, 16).ok()
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transport_preference_is_usb_then_bluetooth_then_unknown() {
        assert!(
            DeviceTransport::Usb.preference_rank() < DeviceTransport::Bluetooth.preference_rank()
        );
        assert!(
            DeviceTransport::Bluetooth.preference_rank()
                < DeviceTransport::Unknown.preference_rank()
        );
    }

    #[test]
    fn preferred_pid_precedes_transport_tie_break() {
        let preferred_bluetooth = candidate_preference_key(
            CODEX_MICRO_PID,
            DeviceTransport::Bluetooth,
            Some(CODEX_MICRO_PID),
        );
        let other_usb = candidate_preference_key(
            crate::ids::CREATOR_MICRO_V2_PIDS[0],
            DeviceTransport::Usb,
            Some(CODEX_MICRO_PID),
        );
        assert!(preferred_bluetooth < other_usb);

        let preferred_usb =
            candidate_preference_key(CODEX_MICRO_PID, DeviceTransport::Usb, Some(CODEX_MICRO_PID));
        assert!(preferred_usb < preferred_bluetooth);
    }

    #[test]
    fn matches_vid_pid_block() {
        let sample = r#"
Codex Micro:

  Product ID: 0x8360
  Vendor ID: 0x303a
  Manufacturer: Work Louder
"#;
        let r = match_usb_text(sample);
        assert!(r.present);
        assert_eq!(r.product_id, Some(CODEX_MICRO_PID));
        assert_eq!(r.transport, DeviceTransport::Usb);
    }

    #[test]
    fn ignores_unrelated_espressif() {
        let sample = r#"
ESP Serial:

  Product ID: 0x1001
  Vendor ID: 0x303a
"#;
        assert!(!match_usb_text(sample).present);
    }

    #[test]
    fn matches_name_fallback() {
        let r = match_usb_text("Something Codex Micro attached");
        assert!(r.present);
    }
}

//! Best-effort USB presence probe — does not claim the HID interface.

use crate::ids::{is_supported_pid, CODEX_MICRO_PID, WL_VID};

/// Result of a non-claiming USB probe.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProbeResult {
    pub present: bool,
    /// Best-effort product id when parsed from the host USB listing.
    pub product_id: Option<u16>,
}

impl ProbeResult {
    pub fn absent() -> Self {
        Self {
            present: false,
            product_id: None,
        }
    }
}

/// Probe for a supported Work Louder / Codex Micro USB device.
pub fn probe_usb_micro() -> ProbeResult {
    #[cfg(target_os = "macos")]
    {
        match system_profiler_usb_text(std::time::Duration::from_secs(3)) {
            Some(text) => match_usb_text(&text),
            None => ProbeResult::absent(),
        }
    }
    #[cfg(not(target_os = "macos"))]
    {
        ProbeResult::absent()
    }
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

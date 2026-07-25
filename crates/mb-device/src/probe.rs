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
        if let Some(text) = system_profiler_usb_text(std::time::Duration::from_secs(3)) {
            let result = match_usb_text(&text);
            if result.present {
                return result;
            }
        }
        // `system_profiler SPUSBDataType` returns empty on some macOS builds;
        // `ioreg` still lists attached USB devices.
        if let Some(text) = ioreg_usb_text() {
            return match_ioreg_text(&text);
        }
        ProbeResult::absent()
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

/// Match `ioreg -p IOUSB -l` text (decimal idVendor/idProduct or product name).
pub fn match_ioreg_text(raw: &str) -> ProbeResult {
    let lower = raw.to_ascii_lowercase();

    if let Some(pid) = ioreg_matching_pid(&lower) {
        return ProbeResult {
            present: true,
            product_id: Some(pid),
        };
    }

    if lower.contains("codex micro") || (lower.contains("work louder") && lower.contains("codex")) {
        return ProbeResult {
            present: true,
            product_id: Some(CODEX_MICRO_PID),
        };
    }

    ProbeResult::absent()
}

/// Find a supported VID/PID pair in `ioreg -p IOUSB -l` output.
///
/// Scoped per device node (`ioreg` starts each with `+-o <name>`) for the same
/// reason [`match_usb_text`] scopes to a record block: the ids only mean
/// anything together. A flat line scan would pair one device's vendor with the
/// next device's product — and would depend on `ioreg` emitting `idVendor`
/// before `idProduct`, which is a property-dictionary order it does not promise.
fn ioreg_matching_pid(lower: &str) -> Option<u16> {
    // `split` on the node marker rather than `lines`, so the first fragment is
    // the (device-less) header and every other fragment is exactly one node.
    lower.split("+-o ").find_map(ioreg_node_matching_pid)
}

fn ioreg_node_matching_pid(node: &str) -> Option<u16> {
    let mut vid = None;
    let mut pid = None;
    for line in node.lines() {
        let line = line.trim();
        // A malformed value must skip that property, never abort the scan: the
        // Codex Micro may be listed after whatever failed to parse.
        if let Some(rest) = line.strip_prefix("\"idvendor\" =") {
            vid = vid.or_else(|| parse_ioreg_id(rest));
        } else if let Some(rest) = line.strip_prefix("\"idproduct\" =") {
            pid = pid.or_else(|| parse_ioreg_id(rest));
        }
    }
    if vid != Some(WL_VID) {
        return None;
    }
    pid.filter(|pid| is_supported_pid(*pid))
}

/// `ioreg -l` prints these as decimal, but tolerate a `0x` form too rather than
/// silently treating a hex listing as "no device attached".
fn parse_ioreg_id(raw: &str) -> Option<u16> {
    let value = raw.trim();
    if let Some(hex) = value.strip_prefix("0x") {
        return u16::from_str_radix(hex, 16).ok();
    }
    value.parse::<u16>().ok()
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
fn ioreg_usb_text() -> Option<String> {
    use std::process::{Command, Stdio};

    let output = Command::new("/usr/sbin/ioreg")
        .args(["-p", "IOUSB", "-l"])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&output.stdout).into_owned())
}

#[cfg(target_os = "macos")]
fn system_profiler_usb_text(timeout: std::time::Duration) -> Option<String> {
    use std::io::Read;
    use std::process::{Command, Stdio};
    use std::thread;
    use std::time::Instant;

    let mut child = Command::new("/usr/sbin/system_profiler")
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

    #[test]
    fn matches_ioreg_decimal_ids() {
        let sample = r#"
  | +-o Codex Micro@02100000
  |       "USB Product Name" = "Codex Micro"
  |       "idVendor" = 12346
  |       "idProduct" = 33632
"#;
        let r = match_ioreg_text(sample);
        assert!(r.present);
        assert_eq!(r.product_id, Some(CODEX_MICRO_PID));
    }

    #[test]
    fn matches_ioreg_name_fallback() {
        let sample = r#""kUSBProductString" = "Codex Micro""#;
        assert!(match_ioreg_text(sample).present);
    }

    /// A device whose ids do not parse must not hide devices listed after it.
    #[test]
    fn ioreg_unparseable_id_does_not_abort_the_scan() {
        let sample = r#"
  | +-o SomeHub@01000000
  |       "idVendor" = <unparseable>
  |       "idProduct" = not-a-number
  | +-o Codex Micro@02100000
  |       "idVendor" = 12346
  |       "idProduct" = 33632
"#;
        let r = match_ioreg_text(sample);
        assert!(r.present, "a malformed earlier device hid the Micro");
        assert_eq!(r.product_id, Some(CODEX_MICRO_PID));
    }

    /// Ids are only meaningful together, so they must come from one node.
    #[test]
    fn ioreg_does_not_pair_ids_across_devices() {
        let sample = r#"
  | +-o WorkLouderOther@01000000
  |       "idVendor" = 12346
  |       "idProduct" = 4097
  | +-o UnrelatedVendor@02100000
  |       "idProduct" = 33632
"#;
        // The supported PID belongs to a node with no Work Louder vendor id.
        assert!(!match_ioreg_text(sample).present);
    }

    #[test]
    fn ioreg_tolerates_property_order_and_hex() {
        let sample = r#"
  | +-o Codex Micro@02100000
  |       "idProduct" = 33632
  |       "idVendor" = 12346
"#;
        assert!(match_ioreg_text(sample).present, "idProduct listed first");

        let hex = r#"
  | +-o Codex Micro@02100000
  |       "idVendor" = 0x303a
  |       "idProduct" = 0x8360
"#;
        assert!(match_ioreg_text(hex).present, "hex-formatted ids");
    }
}

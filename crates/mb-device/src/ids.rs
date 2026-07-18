//! USB identity for Work Louder / Codex Micro devices.
//!
//! Sourced from ChatGPT Desktop's bundled `@worklouder/wl-device-kit`
//! (`DEVICE_REGISTRY` / `WL_VID`). See `docs/device-hid.md`.

/// Work Louder / Espressif USB vendor ID (`0x303A`).
pub const WL_VID: u16 = 0x303A;

/// Codex Micro (firmware name `project_2077`) product ID (`0x8360`).
pub const CODEX_MICRO_PID: u16 = 0x8360;

/// Creator Micro V2 product IDs (same family; not Codex Micro branding).
pub const CREATOR_MICRO_V2_PIDS: [u16; 2] = [0x8297, 0x8298];

/// Vendor-specific HID usage page used for the JSON-RPC channel (`0xFF00`).
pub const WL_USAGE_PAGE: u16 = 0xFF00;

/// Manufacturer strings reported by Work Louder HID devices.
pub const WL_MANUFACTURERS: [&str; 2] = ["Work Louder", "Work_Louder"];

/// Product IDs Microbridge treats as a compatible macropad.
pub fn is_supported_pid(pid: u16) -> bool {
    pid == CODEX_MICRO_PID || CREATOR_MICRO_V2_PIDS.contains(&pid)
}

/// Human label for a supported PID.
pub fn product_name(pid: u16) -> &'static str {
    match pid {
        CODEX_MICRO_PID => "codex-micro",
        0x8297 | 0x8298 => "creator-micro-v2",
        _ => "work-louder",
    }
}

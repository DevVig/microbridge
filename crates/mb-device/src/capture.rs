//! Interactive HID capture for hardware bring-up.
//!
//! Enabled with `--features hid`. Opens the Work Louder vendor interface and
//! streams decoded device→host notifications so the shipping key-string map can
//! be filled in without guessing. This is the tool the
//! [hardware bring-up runbook](../../docs/hardware-bringup.md) drives on day one
//! with a real Codex Micro.
//!
//! Nothing here writes to the device — it is read-only observation.

#![cfg(feature = "hid")]

use std::collections::BTreeMap;
use std::time::{Duration, Instant};

use crate::claim::open_device;
use crate::rpc::DeviceNotify;

/// Rolling stats for a distinct `v.oai.hid` key string.
#[derive(Debug, Default)]
struct KeyStat {
    count: usize,
    last_act: Option<i64>,
    last_agent: Option<i64>,
}

/// Open the Micro and stream decoded device→host events for `seconds`
/// (`0` = until interrupted). Prints a fill-in-the-blank summary at the end so
/// the observed key strings can be dropped straight into the bring-up runbook.
///
/// Read-only: this never claims write ownership or drives LEDs. macOS opens the
/// interface non-exclusively, so quit ChatGPT Desktop and pause `microbridged`'s
/// LED claim first if you want a clean stream.
pub fn run_capture(seconds: u64) -> Result<(), String> {
    let mut device = open_device(None)?;
    let limit = if seconds == 0 {
        "∞".to_string()
    } else {
        format!("{seconds}s")
    };

    println!(
        "microbridge hid-capture — {} (0x{:04X})",
        device.name, device.product_id
    );
    println!("Read-only. Press every Agent Key, then Approve / Reject / Interrupt,");
    println!("rotate the dial, press the dial, and flick the joystick in each direction.");
    println!("Every device→host event prints below. Runs for {limit} (Ctrl-C stops early).\n");

    let start = Instant::now();
    let mut keys: BTreeMap<String, KeyStat> = BTreeMap::new();
    let mut joystick_samples = 0usize;
    let mut others: BTreeMap<String, usize> = BTreeMap::new();

    loop {
        for notify in device.poll_notifies() {
            let t = start.elapsed().as_secs_f32();
            match notify {
                DeviceNotify::Hid { key, act, agent } => {
                    println!("[{t:>6.1}s] hid    k={key:<12} act={act:?} ag={agent:?}");
                    let stat = keys.entry(key).or_default();
                    stat.count += 1;
                    stat.last_act = act;
                    stat.last_agent = agent;
                }
                DeviceNotify::Joystick { angle, distance } => {
                    println!("[{t:>6.1}s] joy    a={angle:?} d={distance:?}");
                    joystick_samples += 1;
                }
                DeviceNotify::Other { method } => {
                    println!("[{t:>6.1}s] other  method={method}");
                    *others.entry(method).or_insert(0) += 1;
                }
            }
        }

        if seconds != 0 && start.elapsed() >= Duration::from_secs(seconds) {
            break;
        }
        // Foreground debug tool: a short sleep keeps CPU near-idle while polling.
        std::thread::sleep(Duration::from_millis(5));
    }

    print_summary(&keys, joystick_samples, &others);
    Ok(())
}

fn print_summary(
    keys: &BTreeMap<String, KeyStat>,
    joystick_samples: usize,
    others: &BTreeMap<String, usize>,
) {
    println!("\n──────── capture summary ────────");
    if keys.is_empty() && joystick_samples == 0 && others.is_empty() {
        println!("No device→host events observed. Is the interface owned by another app");
        println!("(ChatGPT Desktop / microbridged)? Quit it and re-run.");
        return;
    }

    if !keys.is_empty() {
        println!("\nkey (v.oai.hid) → drop these into docs/hardware-bringup.md:");
        println!("  {:<14} {:>5}  last_act  last_ag", "k", "hits");
        for (key, stat) in keys {
            println!(
                "  {:<14} {:>5}  {:<8}  {}",
                key,
                stat.count,
                stat.last_act
                    .map(|a| a.to_string())
                    .unwrap_or_else(|| "-".into()),
                stat.last_agent
                    .map(|a| a.to_string())
                    .unwrap_or_else(|| "-".into()),
            );
        }
    }

    if joystick_samples > 0 {
        println!("\njoystick (v.oai.rad): {joystick_samples} samples");
    }

    if !others.is_empty() {
        println!("\nother notifications (unmapped methods):");
        for (method, count) in others {
            println!("  {method:<20} {count}");
        }
    }
    println!("\nNext: record the mapping in docs/hardware-bringup.md and update");
    println!("`agent_key_index` in crates/mb-device/src/lib.rs if the real strings differ.");
}

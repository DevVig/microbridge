//! HID report framing for the Work Louder RPC channel.
//!
//! Wire layout (64-byte interrupt report), from `wl-device-kit`:
//!
//! ```text
//! [0] report id = 0x06
//! [1] channel   = 1 (debug) | 2 (RPC)
//! [2] length    = payload byte count (0..=61)
//! [3..] UTF-8 payload
//! ```

/// HID report identifier used for Work Louder vendor traffic.
pub const REPORT_ID: u8 = 0x06;

/// Debug / log channel (device → host text).
pub const CHANNEL_DEBUG: u8 = 1;

/// JSON-RPC channel (bidirectional).
pub const CHANNEL_RPC: u8 = 2;

/// Maximum UTF-8 payload bytes per 64-byte HID report.
pub const MAX_CHUNK_SIZE: usize = 61;

/// Full HID report size written to the device (includes report id).
pub const REPORT_SIZE: usize = 64;

/// One demultiplexed HID packet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HidPacket {
    pub channel: u8,
    pub payload: Vec<u8>,
}

/// Frame a UTF-8 message into one or more 64-byte HID reports on `channel`.
pub fn frame_message(channel: u8, message: &[u8]) -> Vec<[u8; REPORT_SIZE]> {
    if message.is_empty() {
        let mut report = [0u8; REPORT_SIZE];
        report[0] = REPORT_ID;
        report[1] = channel;
        report[2] = 0;
        return vec![report];
    }

    let mut out = Vec::new();
    let mut offset = 0;
    while offset < message.len() {
        let chunk = (message.len() - offset).min(MAX_CHUNK_SIZE);
        let mut report = [0u8; REPORT_SIZE];
        report[0] = REPORT_ID;
        report[1] = channel;
        report[2] = chunk as u8;
        report[3..3 + chunk].copy_from_slice(&message[offset..offset + chunk]);
        out.push(report);
        offset += chunk;
    }
    out
}

/// Convenience: frame a string on the RPC channel.
pub fn frame_rpc(message: &str) -> Vec<[u8; REPORT_SIZE]> {
    frame_message(CHANNEL_RPC, message.as_bytes())
}

/// Parse a raw HID read buffer into channel + payload.
///
/// Accepts buffers with or without a leading report id (some backends strip it).
pub fn parse_report(data: &[u8]) -> Option<HidPacket> {
    if data.len() < 3 {
        return None;
    }
    let (channel, length, payload_start) = if data[0] == REPORT_ID {
        if data.len() < 3 {
            return None;
        }
        (data[1], data[2] as usize, 3usize)
    } else {
        // Report id already stripped — treat byte 0 as channel.
        (data[0], data[1] as usize, 2usize)
    };
    if payload_start + length > data.len() || length > MAX_CHUNK_SIZE {
        return None;
    }
    Some(HidPacket {
        channel,
        payload: data[payload_start..payload_start + length].to_vec(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frames_short_rpc_message() {
        let msg = r#"{"method":"sys.version","params":null,"id":1}"#;
        let reports = frame_rpc(msg);
        assert_eq!(reports.len(), 1);
        assert_eq!(reports[0][0], REPORT_ID);
        assert_eq!(reports[0][1], CHANNEL_RPC);
        assert_eq!(reports[0][2] as usize, msg.len());
        assert_eq!(&reports[0][3..3 + msg.len()], msg.as_bytes());
    }

    #[test]
    fn splits_long_messages() {
        let msg = "x".repeat(130);
        let reports = frame_rpc(&msg);
        assert_eq!(reports.len(), 3);
        assert_eq!(reports[0][2] as usize, 61);
        assert_eq!(reports[1][2] as usize, 61);
        assert_eq!(reports[2][2] as usize, 8);
    }

    #[test]
    fn round_trips_report() {
        let reports = frame_rpc("hello");
        let packet = parse_report(&reports[0]).expect("parse");
        assert_eq!(packet.channel, CHANNEL_RPC);
        assert_eq!(packet.payload, b"hello");
    }
}

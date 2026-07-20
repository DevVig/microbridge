//! Canonical IDE/app families for focus + Agent Key scoping.
//!
//! Session `app` labels are stable (`"T3 Code"`, `"Cursor"`, …) while macOS
//! frontmost names often carry channel suffixes (`"T3 Code (Nightly)"`) or
//! casual aliases (`"T3 Chat"`). Exact string equality breaks `focused_app`;
//! compare via [`same_app`] instead.

/// True when two app names refer to the same IDE family.
pub fn same_app(a: &str, b: &str) -> bool {
    if a == b {
        return true;
    }
    app_family(a) == app_family(b)
}

/// Collapse display / frontmost names to a stable family key.
pub fn app_family(name: &str) -> String {
    let trimmed = name.trim();
    // Common canonical session labels — skip lowercasing/allocation work.
    if let Some(family) = canonical_family(trimmed) {
        return family.into();
    }

    let base = strip_channel_suffix(trimmed);
    if let Some(family) = canonical_family(base) {
        return family.into();
    }

    let lower = base.to_ascii_lowercase();
    if is_t3(&lower) {
        return "t3".into();
    }
    if lower == "cursor" || lower.starts_with("cursor ") {
        return "cursor".into();
    }
    if lower == "synara" || lower.starts_with("synara ") {
        return "synara".into();
    }
    if lower == "cnvs" || lower.starts_with("cnvs ") {
        return "cnvs".into();
    }
    if lower == "opencode" || lower.starts_with("opencode ") {
        return "opencode".into();
    }
    if is_chatgpt(&lower) {
        return "chatgpt".into();
    }
    if is_codex(&lower) {
        return "codex".into();
    }
    if is_claude_code(&lower) {
        return "claude_code".into();
    }
    if lower == "claude desktop" || lower.starts_with("claude desktop") {
        return "claude_desktop".into();
    }
    // "Claude Agent SDK" stays its own label (unknown embedders).
    lower
}

fn canonical_family(name: &str) -> Option<&'static str> {
    match name {
        "T3 Code" => Some("t3"),
        "Cursor" => Some("cursor"),
        "Synara" => Some("synara"),
        "Conductor" => Some("conductor"),
        "Factory" => Some("factory"),
        "CNVS" => Some("cnvs"),
        "OpenCode" => Some("opencode"),
        "Codex CLI" => Some("codex"),
        "ChatGPT" | "Codex Desktop" => Some("chatgpt"),
        "Claude Code" => Some("claude_code"),
        "Claude Desktop" => Some("claude_desktop"),
        _ => None,
    }
}

fn strip_channel_suffix(name: &str) -> &str {
    // "T3 Code (Nightly)", "T3 Code (Alpha)", "Cursor (Dev)", …
    if let Some(open) = name.rfind(" (") {
        if name.ends_with(')') && open > 0 {
            return &name[..open];
        }
    }
    name
}

fn is_t3(lower: &str) -> bool {
    matches!(
        lower,
        "t3" | "t3 code" | "t3chat" | "t3 chat" | "t3code" | "t3-code"
    ) || lower.starts_with("t3 code")
        || lower.starts_with("t3 chat")
}

fn is_chatgpt(lower: &str) -> bool {
    matches!(lower, "chatgpt" | "codex app" | "codex desktop")
        || lower.starts_with("chatgpt ")
        || lower.starts_with("codex desktop")
}

fn is_codex(lower: &str) -> bool {
    matches!(lower, "codex" | "codex cli") || lower.starts_with("codex cli")
}

/// Frontmost often reports bare `"Claude"` while sessions are `"Claude Code"`.
fn is_claude_code(lower: &str) -> bool {
    matches!(
        lower,
        "claude" | "claude code" | "claudecode" | "claude-code"
    ) || (lower.starts_with("claude code")
        && !lower.contains("desktop")
        && !lower.contains("agent sdk"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn t3_nightly_matches_t3_code() {
        assert!(same_app("T3 Code (Nightly)", "T3 Code"));
        assert!(same_app("T3 Chat", "T3 Code"));
        assert!(same_app("t3code", "T3 Code"));
    }

    #[test]
    fn cursor_and_synara_match_themselves() {
        assert!(same_app("Cursor", "Cursor"));
        assert!(same_app("Synara", "Synara"));
        assert!(!same_app("Cursor", "Synara"));
        assert!(!same_app("Cursor", "T3 Code"));
        assert!(same_app("Conductor", "Conductor"));
        assert!(same_app("Factory", "Factory"));
        assert!(same_app("CNVS", "CNVS"));
        assert!(same_app("OpenCode", "OpenCode (Dev)"));
    }

    #[test]
    fn codex_cli_matches_codex_frontmost() {
        assert!(same_app("Codex", "Codex CLI"));
        assert!(same_app("Codex CLI", "Codex CLI"));
        assert!(!same_app("ChatGPT", "Codex CLI"));
        assert!(same_app("Codex Desktop", "ChatGPT"));
    }

    #[test]
    fn claude_frontmost_matches_claude_code() {
        assert!(same_app("Claude", "Claude Code"));
        assert!(same_app("Claude Code (Nightly)", "Claude Code"));
        assert!(!same_app("Claude Code", "Claude Desktop"));
        assert!(!same_app("Claude", "Claude Desktop"));
        assert!(!same_app("Claude Code", "Claude Agent SDK"));
        assert!(!same_app("Claude Code", "Synara"));
    }
}

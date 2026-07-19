//! Canonical IDE/app families for focus + Agent Key scoping.
//!
//! Session `app` labels are stable (`"T3 Code"`, `"Cursor"`, …) while macOS
//! frontmost names often carry channel suffixes (`"T3 Code (Nightly)"`) or
//! casual aliases (`"T3 Chat"`). Exact string equality breaks `focused_app`;
//! compare via [`same_app`] instead.

/// True when two app names refer to the same IDE family.
pub fn same_app(a: &str, b: &str) -> bool {
    app_family(a) == app_family(b)
}

/// Collapse display / frontmost names to a stable family key.
pub fn app_family(name: &str) -> String {
    let trimmed = name.trim();
    let base = strip_channel_suffix(trimmed);
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

fn is_codex(lower: &str) -> bool {
    matches!(lower, "codex" | "codex cli" | "codex app" | "chatgpt") || lower.starts_with("codex ")
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
    }

    #[test]
    fn codex_cli_matches_codex_frontmost() {
        assert!(same_app("Codex", "Codex CLI"));
        assert!(same_app("Codex CLI", "Codex CLI"));
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

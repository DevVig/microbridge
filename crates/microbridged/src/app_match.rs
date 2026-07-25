//! Canonical IDE/app families for focus + Agent Key scoping.
//!
//! Session `app` labels are stable (`"T3 Code"`, `"Cursor"`, …) while macOS
//! frontmost names often carry channel suffixes (`"T3 Code (Nightly)"`) or
//! casual aliases (`"T3 Chat"`). Exact string equality breaks `focused_app`;
//! compare via [`same_app`] instead.
//!
//! The matching itself lives in `mb_protocol::ide` so the menu bar app resolves
//! families exactly as the daemon does. These are the daemon-side names for it.

/// True when two app names refer to the same IDE family.
pub fn same_app(a: &str, b: &str) -> bool {
    if a == b {
        return true;
    }
    app_family(a) == app_family(b)
}

/// Collapse display / frontmost names to a stable family key.
pub fn app_family(name: &str) -> String {
    mb_protocol::ide::family_for_app(name)
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

    /// The registry claims `Ide::family` equals `app_family(Ide::label)`. If that
    /// ever stops holding, a pinned controller silently matches no session — so
    /// assert it rather than trusting the two tables to stay in step.
    #[test]
    fn every_registered_ide_round_trips_label_to_family() {
        for ide in mb_protocol::IDES {
            assert_eq!(
                app_family(ide.label),
                ide.family,
                "{} label does not resolve to its own family",
                ide.label
            );
            for alias in ide.aliases {
                assert_eq!(app_family(alias), ide.family, "alias {alias}");
            }
        }
    }

    /// The menu bar app resolves families through the same `mb_protocol::ide`
    /// entry point, so a pinned IDE can never look live to one side and idle to
    /// the other.
    #[test]
    fn daemon_and_ui_share_one_matcher() {
        for name in [
            "T3 Code",
            "T3 Code (Nightly)",
            "Cursor Agent (ACP)",
            "Claude",
            "Codex",
            "Claude Agent SDK",
        ] {
            assert_eq!(app_family(name), mb_protocol::ide::family_for_app(name));
        }
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

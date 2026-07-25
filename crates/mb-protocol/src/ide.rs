//! Canonical IDE registry — one table for a concept that had three vocabularies.
//!
//! The same IDE was previously named three different ways in three places: the
//! session `app` label (`"T3 Code"`), the focus family key (`"t3"`), and the
//! adapter id (`"t3code"`). Nothing tied them together, so they drifted.
//!
//! [`IDES`] is now the single source of truth. `app_match::canonical_family`
//! looks up labels here instead of keeping its own table, `DaemonConfig`
//! validates `controlling_ide` against it, and the menu bar app orders its
//! "Controlled by" submenu from it.
//!
//! # Why `providers` is a list
//!
//! Hosts and harnesses are many-to-many, and that is the whole reason the
//! controller has to be picked rather than inferred. A T3 Code session can
//! reach the bus from the `t3code` paired-HTTP control plane *or* from the
//! `codex` journal watcher (`originator: t3code…`) *or* from the `claude`
//! journal watcher (an Agent SDK session under `~/.t3/`). All three are the
//! same IDE to the user, so all three are listed under one family.

use crate::Action;

/// What dial rotation means for the focused thread's IDE.
///
/// There is deliberately no `Off` variant. Whether an IDE can act on a lever is
/// already answered dynamically and correctly by `AdapterCapabilities`; encoding
/// it a second time as a static per-IDE fact would only add something new to go
/// stale the moment a host gains the capability.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DialRole {
    /// Rotate to step reasoning effort (the historical behavior).
    Effort,
    /// Rotate to move through the IDE's own navigation surface.
    Navigate,
}

/// What a joystick flick means for the focused thread's IDE.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JoystickRole {
    /// Cycle the local deck selection (the historical behavior).
    DeckCycle,
    /// Send navigation to the owning adapter.
    Navigate,
}

/// Per-IDE physical input behavior.
///
/// The deck's input map used to be one hardcoded `match` that had to behave
/// identically everywhere, because the daemon could not be sure which IDE it
/// was talking to. A pinned controller removes that doubt, so the varying parts
/// live here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IdeProfile {
    pub dial: DialRole,
    pub joystick: JoystickRole,
    pub dial_press: Action,
}

/// The behavior every IDE had before profiles existed. Any IDE without an
/// explicit profile keeps exactly this, so adding the seam changed nothing.
pub const DEFAULT_PROFILE: IdeProfile = IdeProfile {
    dial: DialRole::Effort,
    joystick: JoystickRole::DeckCycle,
    dial_press: Action::OpenFocusedThread,
};

/// One IDE family, and everything that identifies it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ide {
    /// Stable internal key. Must equal `app_match::app_family(self.label)`.
    pub family: &'static str,
    /// Canonical session `app` label, and the "Controlled by" menu title.
    pub label: &'static str,
    /// Additional exact labels that mean the same IDE. Fuzzy matching (channel
    /// suffixes like `"T3 Code (Nightly)"`, casual aliases like `"T3 Chat"`)
    /// stays in `app_match`; this is only for distinct canonical spellings.
    pub aliases: &'static [&'static str],
    /// Adapter ids that can produce sessions for this IDE. Every entry must
    /// exist in the daemon's adapter registry.
    pub providers: &'static [&'static str],
    pub profile: &'static IdeProfile,
}

/// Menu order mirrors `INTEGRATION_ORDER` in the Settings surface, collapsed to
/// families — which is why Cursor appears once here despite having two adapters.
pub const IDES: &[Ide] = &[
    Ide {
        family: "chatgpt",
        label: "ChatGPT",
        aliases: &["Codex Desktop"],
        providers: &["chatgpt", "codex"],
        profile: &DEFAULT_PROFILE,
    },
    Ide {
        family: "claude_desktop",
        label: "Claude Desktop",
        aliases: &[],
        providers: &["claude_desktop", "claude"],
        profile: &DEFAULT_PROFILE,
    },
    Ide {
        family: "claude_code",
        label: "Claude Code",
        aliases: &[],
        providers: &["claude"],
        profile: &DEFAULT_PROFILE,
    },
    Ide {
        family: "codex",
        label: "Codex CLI",
        aliases: &[],
        providers: &["codex"],
        profile: &DEFAULT_PROFILE,
    },
    Ide {
        family: "cnvs",
        label: "CNVS",
        aliases: &[],
        providers: &["cnvs"],
        profile: &DEFAULT_PROFILE,
    },
    Ide {
        family: "synara",
        label: "Synara",
        aliases: &[],
        providers: &["synara", "codex", "claude"],
        profile: &DEFAULT_PROFILE,
    },
    Ide {
        family: "conductor",
        label: "Conductor",
        aliases: &[],
        providers: &["conductor", "codex", "claude"],
        profile: &DEFAULT_PROFILE,
    },
    // Cursor keeps the default profile on purpose. The `cursor` adapter is
    // lifecycle-only hooks and `cursor_acp` explicitly does not remote-control
    // an already-open composer, so there is no navigation surface to bind — and
    // its dial behavior is already correct via capability negotiation. Revisit
    // when Cursor ships a public navigation surface; synthesizing UI keystrokes
    // is deliberately not on the table.
    Ide {
        family: "cursor",
        label: "Cursor",
        aliases: &["Cursor Agent (ACP)"],
        providers: &["cursor", "cursor_acp", "codex", "claude"],
        profile: &DEFAULT_PROFILE,
    },
    Ide {
        family: "t3",
        label: "T3 Code",
        aliases: &[],
        providers: &["t3code", "codex", "claude"],
        profile: &DEFAULT_PROFILE,
    },
    Ide {
        family: "factory",
        label: "Factory",
        aliases: &[],
        providers: &["factory", "codex"],
        profile: &DEFAULT_PROFILE,
    },
    Ide {
        family: "opencode",
        label: "OpenCode",
        aliases: &[],
        providers: &["opencode"],
        profile: &DEFAULT_PROFILE,
    },
];

pub fn ide_for_family(family: &str) -> Option<&'static Ide> {
    IDES.iter().find(|ide| ide.family == family)
}

/// Exact label → family, for the canonical spellings in [`IDES`].
///
/// Prefer [`family_for_app`] unless you specifically want exact matching.
pub fn family_for_label(label: &str) -> Option<&'static str> {
    IDES.iter()
        .find(|ide| ide.label == label || ide.aliases.contains(&label))
        .map(|ide| ide.family)
}

/// Collapse any display name to a stable family key.
///
/// Adapters emit the canonical labels in [`IDES`], but macOS frontmost names
/// carry channel suffixes (`"T3 Code (Nightly)"`) and casual aliases
/// (`"T3 Chat"`, bare `"Claude"`), so exact equality is not enough.
///
/// This lives here rather than in the daemon because the menu bar app has to
/// decide whether a pinned IDE has live sessions and must reach the *same*
/// answer the daemon's focus policy did — two matchers that can disagree would
/// show a lock as inactive while the daemon was honoring it.
///
/// Unrecognized names collapse to their lowercased selves, so an unknown
/// embedder keeps a stable identity of its own rather than being folded into
/// someone else's family.
pub fn family_for_app(name: &str) -> String {
    let trimmed = name.trim();
    // Canonical labels are the common case — skip lowercasing/allocation work.
    if let Some(family) = family_for_label(trimmed) {
        return family.into();
    }

    let base = strip_channel_suffix(trimmed);
    if let Some(family) = family_for_label(base) {
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

/// Menu title for a family, falling back to the raw key so an unknown value is
/// visible rather than silently blank.
pub fn label_for_family(family: &str) -> &str {
    ide_for_family(family).map_or(family, |ide| ide.label)
}

pub fn is_known_family(family: &str) -> bool {
    ide_for_family(family).is_some()
}

/// Profile for a family; unknown families get the historical behavior.
pub fn profile_for_family(family: &str) -> &'static IdeProfile {
    ide_for_family(family).map_or(&DEFAULT_PROFILE, |ide| ide.profile)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn families_and_labels_are_unique() {
        for (index, ide) in IDES.iter().enumerate() {
            let duplicate = IDES
                .iter()
                .skip(index + 1)
                .any(|other| other.family == ide.family || other.label == ide.label);
            assert!(!duplicate, "duplicate family/label for {}", ide.family);
        }
    }

    #[test]
    fn every_label_and_alias_resolves_to_its_family() {
        for ide in IDES {
            assert_eq!(family_for_label(ide.label), Some(ide.family));
            for alias in ide.aliases {
                assert_eq!(family_for_label(alias), Some(ide.family), "alias {alias}");
            }
        }
        assert_eq!(family_for_label("Nonexistent Editor"), None);
    }

    #[test]
    fn providers_are_never_empty() {
        // A family with no provider could be pinned but never receive a
        // session, which would look like a broken lock.
        for ide in IDES {
            assert!(!ide.providers.is_empty(), "{} has no providers", ide.family);
        }
    }

    #[test]
    fn unknown_family_falls_back_to_default_profile() {
        assert_eq!(profile_for_family("nonexistent"), &DEFAULT_PROFILE);
        assert_eq!(label_for_family("nonexistent"), "nonexistent");
        assert!(!is_known_family("nonexistent"));
    }
}

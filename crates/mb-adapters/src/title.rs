//! Shared helpers for deriving short, human titles from agent journals.

/// Collapse whitespace and truncate for menu-bar display.
pub fn clean_title(raw: &str, max_chars: usize) -> String {
    let collapsed: String = raw.split_whitespace().collect::<Vec<_>>().join(" ");
    let trimmed = collapsed.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    if trimmed.chars().count() <= max_chars {
        return trimmed.to_string();
    }
    let mut out: String = trimmed.chars().take(max_chars.saturating_sub(1)).collect();
    out.push('…');
    out
}

/// Skip system / sandbox boilerplate that isn't a real user ask.
pub fn looks_like_boilerplate(text: &str) -> bool {
    let t = text.trim_start();
    t.starts_with('<')
        || t.starts_with("# Repository Guidelines")
        || t.starts_with("# Project Agents")
        || t.starts_with("Synara plan mode is active")
        || t.starts_with("I'm evaluating whether")
        || t.contains("<environment_context>")
        || t.contains("<permissions instructions>")
        || t.contains("<system_instruction>")
        || t.contains("<handoff_context>")
        || t.contains("<recommended_plugins>")
}

/// Last path component of a cwd, suitable as a fallback title.
pub fn cwd_basename(cwd: &str) -> String {
    std::path::Path::new(cwd)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or(cwd)
        .to_string()
}

/// Decode Claude project folder names like `-Users-me-dev-foo` → `foo`.
pub fn project_label_from_path(path: &std::path::Path) -> String {
    let parent = path
        .parent()
        .and_then(|p| p.file_name())
        .and_then(|s| s.to_str())
        .unwrap_or("project");
    let cleaned = parent.trim_start_matches('-').replace('-', "/");
    cwd_basename(&cleaned)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cleans_and_truncates() {
        assert_eq!(clean_title("  hello   world  ", 32), "hello world");
        assert!(clean_title("abcdefghij", 6).ends_with('…'));
    }

    #[test]
    fn rejects_boilerplate() {
        assert!(looks_like_boilerplate("<environment_context>\nfoo"));
        assert!(!looks_like_boilerplate("Fix the flaky e2e retries"));
    }
}

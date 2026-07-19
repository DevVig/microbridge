//! Naming the app a session is running inside.
//!
//! Both journal stores are shared: `~/.claude/projects` is written by the
//! Claude CLI, Claude Desktop and every app embedding the Agent SDK, and
//! `~/.codex/sessions` by the Codex CLI, the Codex app and every app wrapping
//! `codex app-server`. So neither store's location identifies its host, and
//! both adapters need the same answer for the same question.

/// Map a session `cwd` to the embedding host when it lives under that host's
/// home directory. Purely path-based, so it names the host for worktree
/// sessions without touching the host at all.
///
/// This is the fallback for journals with no `originator`/`entrypoint` field —
/// older ones, mostly. Where those fields exist they are authoritative, since a
/// host can perfectly well open a session outside its own home.
pub(crate) fn host_from_cwd(cwd: Option<&str>) -> Option<&'static str> {
    let cwd = cwd?;
    let home = std::env::var("HOME").ok()?;
    // (home-relative dir, display name) — Synara / T3 Code / Cursor all keep
    // their worktrees under a dot-directory in $HOME. The display name doubles
    // as the `focused_app` scope for Agent Keys, so it has to match the name
    // the rest of the app uses for that IDE.
    const HOSTS: &[(&str, &str)] = &[
        (".synara", "Synara"),
        (".t3", "T3 Code"),
        (".cursor", "Cursor"),
    ];
    for (dir, name) in HOSTS {
        let prefix = format!("{home}/{dir}/");
        if cwd.starts_with(&prefix) {
            return Some(name);
        }
    }
    None
}

# Repository governance

How changes land on `main`, and how macOS users install/update without cloning.

## Branch protection (`main`)

Enforced via GitHub **rulesets** (Settings → Rules):

| Rule | Setting |
|---|---|
| Direct pushes | Blocked (`non_fast_forward` + PR required) |
| Force push / delete | Blocked |
| Merge method | **Squash only** |
| Status checks | `rust (ubuntu-latest)`, `rust (macos-latest)`, `ui` (strict); PR title lint is CI advisory |
| Conversations | Must be resolved |
| Approvals | 0 required (solo-friendly); stale reviews dismissed |
| Admin bypass | Via pull request only |

Release tags matching `v*` cannot be deleted or force-moved (admin bypass allowed).

Repo merge defaults: squash title = PR title, body = PR body, delete head branch
on merge, auto-merge enabled.

## Commit / PR conventions

- PR titles must be [Conventional Commits](https://www.conventionalcommits.org/)
  (`feat:`, `fix:`, `docs:`, `adapter:`, …) — enforced by the **PR title** workflow.
- Squash merge uses the PR title as the commit subject, so `main` history stays
  conventional without a commit-message ruleset (unavailable on this plan).
- See [CONTRIBUTING.md](../CONTRIBUTING.md) and the PR template.

## Releases

1. Land changes on `main` via PR.
2. Tag `vX.Y.Z` and push the tag → **Release** workflow builds archives and
   updates the Homebrew formula checksums.
3. Users upgrade with `brew update && brew upgrade microbridge`.

## macOS install + auto-update (Homebrew)

**This is the supported consumer path** — not cloning the git repo.

```sh
brew tap DevVig/microbridge https://github.com/DevVig/microbridge
brew install microbridge
brew services start microbridge
```

Updates:

```sh
brew update && brew upgrade microbridge
brew services restart microbridge   # if the formula changed
```

Optional background updates (Homebrew’s own updater):

```sh
brew autoupdate start --upgrade --cleanup --immediate
```

Details: [INSTALL.md](../INSTALL.md#homebrew-recommended-on-macos).

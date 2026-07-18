# macOS signing & notarization

Direct-download DMGs are signed with **Developer ID Application** and
notarized via the App Store Connect API. This is **not** Mac App Store
distribution.

## GitHub Actions secrets

| Secret | Purpose |
|---|---|
| `APPLE_CERTIFICATE` | Base64-encoded `.p12` (Developer ID Application + private key) |
| `APPLE_CERTIFICATE_PASSWORD` | Password for that `.p12` |
| `APPLE_SIGNING_IDENTITY` | e.g. `Developer ID Application: Vig Solutions LLC (3NQG568C4Q)` |
| `KEYCHAIN_PASSWORD` | Ephemeral CI keychain password |
| `APPLE_API_KEY` | App Store Connect API Key ID |
| `APPLE_API_ISSUER` | App Store Connect Issuer ID |
| `APPLE_API_KEY_P8` | Contents of the `.p8` private key file |
| `APPLE_TEAM_ID` | Team ID (`3NQG568C4Q`) |

Release tags (`v*`) run `.github/workflows/release.yml`, which builds
`app` + `dmg` bundles with Tauri when those secrets are present.

## Local assets (do not commit)

Developer ID material lives outside the repo, typically under
`~/.asc/signing/developer-id/`.

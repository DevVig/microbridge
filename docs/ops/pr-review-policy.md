# Independent PR review standard

Repository: `DevVig/microbridge`; default branch: `main`; primary language: Rust.

Jon approved this standard on 2026-10-05 for owned active GitHub repositories. It changes reviewer scheduling and fallback only. Repository tests, types, lint, build, security, compatibility, acceptance and production approval requirements remain in force. This policy supersedes vendor-specific mandatory CLI/zero-finding wording for this workflow.

## Final candidate and reviewer selection

1. Run the applicable repository checks during implementation, without waiting for a bot. Keep the PR in draft, batch fixes, and commit the final candidate. Freeze the PR merge base and full candidate SHA.
2. Use one completed CodeRabbit review when it is available and already authorized. Either the existing hosted review or CLI review is sufficient if it covers the complete candidate; do not require both. Avoid repeated requests during a cooldown, and never buy credits or change plans/spend caps to get a review through.
3. After an explicit CodeRabbit cooldown or service/client/authentication error, retain the actual error evidence and start a fresh independent Codex review using the existing authorized local login. A qualified human is also an independent fallback. Bugbot may supply independent review only where existing repository access and authorization are verified. Ordinary PRs do not require all three vendors.
4. A skipped, ignored, rate-limited, queued or failed check is not review completion, regardless of its color. Verify the actual result and reviewed SHA. Missing coverage requires independent review; another vendor name alone does not establish coverage.
5. Any code change invalidates the old candidate receipt. Re-run applicable checks and final review on the new SHA. PR-body metadata updates do not change the code SHA. After a squash merge, record the merge SHA and prove tree equivalence; source drift requires renewed review/checks.

The reviewer must inspect the actual full diff, surrounding contracts, applicable instructions/specifications and earlier finding fixes. The implementation agent's own reread is not independent. Give the reviewer every changed path, including deletions; directly inspect changed visual/binary assets or record a coverage gap.

## Sensitive changes and finding disposition

Billing, authentication/authorization, encryption/credentials, tenant isolation, schema/migrations and review controls require an additional independent review and targeted tests at the final candidate SHA. Include relevant negative tests for denied identities/roles, cross-tenant access, webhook/replay boundaries, encryption/key failures, migration compatibility and rollback as applicable. Use judgment to flag sensitive changes whose path names do not reveal the risk.

Every actionable correctness, security, privacy, data-integrity, compatibility, accessibility or usability defect must be fixed and verified. Record every finding, severity, path/line, reasoning and disposition. Optional wording/style suggestions may be nonblocking with an evidence-backed reason accepted by the independent reviewer. Do not relabel a real defect as cosmetic. Resolve GitHub review conversations with evidence before merge. Record reviewer identity/context, implementation identity/context, timestamp, exact base/candidate SHA, path coverage, raw report and limitations.

## Local helper

`scripts/review/review.py` uses Python 3.9+ and Git. Its `prepare` and `validate` commands work offline. `codex` starts a new non-resumed CLI process with a read-only sandbox and structured report; it uses the existing Codex login and normal account usage. The helper does not install tools, initiate login, forward credentials, request CodeRabbit/Bugbot, post GitHub comments, approve, merge or deploy.

Store evidence outside the worktree so the candidate stays clean. For example:

```sh
REVIEW_DIR="$(git rev-parse --absolute-git-dir)/review/final-candidate"
mkdir -p "$REVIEW_DIR"
# Save the actual CodeRabbit failure/cooldown result to this local file first.
# Use the exact PR merge-base SHA and the actual implementation actor/thread.
python3 -B scripts/review/review.py prepare \
  --base BASE_SHA --candidate HEAD \
  --implementer IMPLEMENTER_ID --implementation-context IMPLEMENTATION_THREAD_ID \
  --coderabbit-state error --fallback-reason "Recorded CodeRabbit client/service error" \
  --fallback-evidence "$REVIEW_DIR/coderabbit-error.txt" \
  --output "$REVIEW_DIR/packet.json"
python3 -B scripts/review/review.py codex \
  --packet "$REVIEW_DIR/packet.json" \
  --reviewer INDEPENDENT_REVIEWER_ID --review-context FRESH_REVIEW_CONTEXT_ID \
  --output "$REVIEW_DIR/codex-review"
python3 -B scripts/review/review.py validate \
  --packet "$REVIEW_DIR/packet.json" \
  --receipt "$REVIEW_DIR/codex-review/receipt.json"
```

The last command fails while findings remain open, coverage is incomplete, the SHA changed, the checkout is dirty, or evidence artifacts changed. For sensitive candidates, pass a second `--receipt` with a distinct independent review context and `--targeted-tests PATH.json`. The test evidence has `candidate_sha`, literal `status: "passed"`, a nonempty `tests` array listing commands/results, and `report` containing an actual local output `path` and its `sha256`.

For completed existing CodeRabbit, Bugbot or human reviews, retain the original result and normalize its final report to `report.schema.json`: literal `status: "completed"`, exact `base_sha`/`candidate_sha`, summary, every `covered_paths`, `limitations` and every finding. Do not normalize an upstream skipped/failed result into completion. Create a receipt using the same fields as the Codex receipt, with the actual provider, reviewer/context, completion timestamp and a hashed report artifact. Bugbot receipts additionally require `existing_authorization` evidence with `path`/`sha256`. The report and receipt must agree on status, scope and findings.

Findings start with `disposition: "open"`. A disposition requires `rationale`, `verification` and `accepted_by` matching the independent reviewer. `nonblocking` is limited to low-severity suggestions; resolved defects still require verification. A receipt cannot drop/alter the original report's findings. Fixing code produces a new candidate and new review; preserve earlier artifacts rather than overwriting them.

```sh
python3 -B -m unittest discover -s scripts/review -p 'test_*.py' -v
```

These are local, auditable evidence checks, not signed attestations or a tamper-proof approval system. The release owner must verify the upstream result, reviewer independence, finding disposition and the complete required CI/acceptance evidence. Success reports review evidence complete and **merge_approval: false**. Never treat it as GitHub approval or a replacement for required checks.

## Activation and GitHub enforcement

After this change is accepted, the repository has an available manual local fallback and a selectable GitHub review-evidence PR template. Existing hosted review settings continue as configured. No bot app, token, OAuth scope, subscription, spending limit, branch protection, ruleset or production setting is changed by this rollout. No credential-based hosted review workflow is added or enabled.

The local helper is active only when a reviewer/operator runs it. The standard is process guidance until accepted by the repository owners. GitHub enforcement remains the repository's existing required checks/reviews; this helper is not a newly required check. Enabling another bot, paid/API CI review, persistent permissions, or a new branch-protection/ruleset requirement needs a concrete proposal and separate approval. Production deployment approval stays separate from review/merge readiness.

Current official references: [Codex CLI commands](https://developers.openai.com/codex/cli/reference/), [CodeRabbit CLI](https://docs.coderabbit.ai/cli/), [CodeRabbit rate limits](https://docs.coderabbit.ai/management/rate-limits), and [Bugbot setup/configuration](https://cursor.com/docs/bugbot). Bugbot's API may require enterprise authorization and bill even dry-run reviews; this helper does not call it.


## Repository gate map

Keep the existing scoped requirements in `AGENTS.md`, native/module instructions, and GitHub workflows. This list records available root commands, not a claim that they all ran or that every script is required for every change.

No generic root Node test gate was inferred. Use the existing native/static/manual gates for the changed module; record unavailable checks explicitly.

Preserve the Rust/Cargo and desktop/native checks in `.github/workflows/ci.yml`, plus release/package verification. No Node test gate is invented for Rust code.

Existing workflow files remain unchanged:

- `.github/workflows/release.yml`
- `.github/workflows/finalize-release.yml`
- `.github/workflows/pr-title.yml`
- `.github/workflows/ci.yml`

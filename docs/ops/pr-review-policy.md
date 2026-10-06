# PR review and cost controls

Copied from AllureConnect's PR review policy (approved September 6, 2026, Eastern time) with the October 5, 2026 automatic Codex fallback addendum. This is the current review policy for Microbridge. It supersedes older instructions requiring a particular vendor's zero-finding result; historical review reports remain accurate records of their own checkpoints. It does not amend product implementation plans or change the tag-driven release procedure.

## Review the completed candidate

1. Keep work in draft while implementing. Run the applicable local checks, freeze the intended diff, and batch review fixes in one push. Do not push during an active review unless correcting an urgent problem; record that the old review is superseded.
2. CodeRabbit remains the normal reviewer. Automatic review of ready PRs remains enabled. This repository has no `.coderabbit.yaml`, so CodeRabbit defaults apply: [automatic incremental review on each push is on](https://docs.coderabbit.ai/configuration/auto-review), and drafts are skipped unless configured otherwise. Request a final review explicitly after subsequent substantive fixes if the incremental pass did not cover the final candidate. An opening review is sufficient only if it covers the final candidate.
3. Use one coordinated review request at a time for the shared CodeRabbit CLI identity. Do not repeatedly retry during a reported cooldown. Do not buy credits, upgrade plans, or increase spending caps without explicit budget authorization.
4. A green check named skipped, rate limited, or ignored is not evidence that a review ran. Read the actual result, scope and revision. Do not require both hosted and CLI reviews if one completed review covers the whole candidate; missing image/binary coverage must be supplied by an independent visual reviewer.

CodeRabbit's supported automatic review controls and review accounting are documented in its [configuration reference](https://docs.coderabbit.ai/reference/configuration#auto_review) and [rate-limit guidance](https://docs.coderabbit.ai/management/rate-limits). No review path filters, scan thresholds or test assertions are loosened by this policy.

## Independent fallback

When CodeRabbit reports a rate limit, cannot start or complete because of a service/client failure, or cannot cover a changed artifact, record that condition and use an independent reviewer. A reviewer may be a separate Codex review session or agent, a qualified human, or another approved review service. The implementation agent's own reread is not independent review. Existing Codex review uses the account's normal usage allowance; it is not unlimited or an assurance of zero additional usage cost.

Give the reviewer the frozen base/head and full changed-path inventory, requirements, applicable repository instructions and tests. The reviewer inspects the actual code and relevant surrounding contracts, checks earlier findings and fixes, and directly views changed visual assets. A second provider name alone does not establish completeness. Do not disclose secrets or customer data in prompts, screenshots or retained evidence.

Record the fallback honestly as an independent review, never as `CodeRabbit: 0`. A provider outage does not waive unresolved findings or permit merging an incomplete review. For auth, billing, credentials, schema, public API or integration changes, the independent review must explicitly evaluate the changed security and compatibility boundaries as well as the relevant negative tests.

### Automatic Codex fallback

Jonathan approved this automation on October 5, 2026: request a Codex review automatically only when CodeRabbit cannot review a ready pull request. The workflow is `.github/workflows/coderabbit-fallback-review.yml` and the decision script is `.github/pr-review-fallback/pr-review-fallback.mjs`. Unit tests live at `scripts/pr-review-fallback.test.mjs` so `node --test` (which skips hidden `.github/` directories) actually runs them; CI invokes that file explicitly because this repository does not run a repo-root `node --test`. It is fallback-only and must not run on every pull request. The workflow token is read-only plus checks/statuses read, and all writes use `CODEX_GITHUB_TOKEN`.

Triggers:

- A CodeRabbit comment that reports a rate limit (`rate limited by coderabbit.ai` / `Review limit reached`) or a failure (`failure by coderabbit.ai` / `Review failed`), including the passing `CodeRabbit` commit status titled `Review rate limited`.
- A scheduled hourly sweep (`59 * * * *`) that drains a backlog a few pull requests at a time.
- `workflow_dispatch` with `dry_run` (default on) to list which pull requests would get a fallback and why, without commenting.

Window and throttle (defaults, overridable at the top of the workflow):

- Window: **45 minutes** after the pull request became ready (`ready_for_review`, or `opened` if it was never a draft). The window is not restarted by later pushes.
- This repository has no `.coderabbit.yaml`, so CodeRabbit's default `auto_incremental_review: true` applies (review on each push). The fallback script still treats a completed CodeRabbit review on an earlier head as “CodeRabbit reviewed” unless someone later posted `@coderabbitai review` or `@coderabbitai full review`. That is conservative: it will not request Codex after the first successful CodeRabbit review just because later commits arrived, even if an incremental CodeRabbit pass later rate-limits or fails. First-time ready pull requests that never get a completed CodeRabbit review still fall through to Codex after a rate limit, error, or the 45-minute window. After an explicit re-request, the window starts at the request time.
- Throttle: **2 pull requests per run per repository**. Oldest eligible first. At most one fallback per pull-request head SHA.

Skip rules: draft pull requests; dependency bots (`dependabot[bot]`, `renovate[bot]`); a fallback marker that already exists for the current head; a completed CodeRabbit review that covers the current head; a CodeRabbit check still in progress.

How the fallback is recorded: the workflow posts one comment that starts with `@codex review` and includes an HTML marker (`pr-review-fallback:sha:<head>` plus the reason) and applies the label `review-fallback-coderabbit-unavailable`. The comment states that CodeRabbit was unavailable and that Codex is the independent fallback. It is never recorded as a CodeRabbit pass.

Codex trigger and credential: official Codex docs request a review by commenting `@codex review` on the pull request (ChatGPT Codex GitHub connector). Codex maps the comment author to a ChatGPT-connected GitHub account, so `github-actions[bot]` / `GITHUB_TOKEN` mentions are not a reliable trigger. Set repository secret `CODEX_GITHUB_TOKEN` to a fine-grained PAT (issues and pull requests: write) for the GitHub user whose ChatGPT account has this repository connected and code review enabled at `https://chatgpt.com/codex/settings/code-review`. The ChatGPT Codex Connector GitHub App must have access to the repository. Until that secret is present, scheduled and comment-driven runs that would fire fail closed and quiet: they post nothing, emit a workflow warning plus a job-summary line of the pull requests that would have gotten a fallback, and exit 0 so the hourly sweep does not fail. Genuine script or GitHub API errors still fail the job.

Bugbot (Cursor) stays manual-only. Do not add Bugbot auto-review configuration and do not have this workflow mention Bugbot. When Jonathan asks for a Bugbot pass on a specific pull request, post a new **top-level** comment on that pull request containing `cursor review` or `bugbot run` (not a thread reply). That is the only automatic-adjacent Bugbot action this policy permits, and it is still a human action.

## Required evidence and finding disposition

The review packet, linked from the PR body, must include:

- Exact base and candidate commit, review timestamp, reviewer identity/tool, and the implementation author so independence is assessable.
- Every changed path and whether it received code review, visual inspection, or another justified form of review. No changed path may silently disappear through tool defaults or filters.
- CodeRabbit failure/rate-limit evidence when fallback was used, without tokens or other secrets.
- Findings with file/line, severity, concrete reasoning or reproduction, and disposition. Each valid actionable defect must be fixed and verified. Disputed findings need an evidence-backed rationale accepted by the independent reviewer and release owner.
- Explicit limitations: checks not run, routes or providers not exercised, and unverified deployment/authentication evidence. Keep wider rollout gates separate from the scoped merge verdict.
- A final review verdict and applicable exact-commit CI results. Preserve initial failures and retry counts; a retry pass is not a first-pass pass.

The merge verdict requires no unresolved actionable correctness, security, privacy, data-integrity, compatibility, accessibility or usability defects. Optional wording/style suggestions may be recorded as nonblocking with a reason and an owner for any follow-up; they must not force indefinite full-review cycles. Do not relabel a real defect as cosmetic or resolve a thread merely to obtain green status. Every review conversation must have a documented disposition and be resolved before merge.

After substantive changes, independently review the new diff and its interactions before merging. Earlier review coverage may be retained only when exact reviewed paths are unchanged and that equivalence is recorded; review all integration/conflict changes. Metadata-only PR-body updates do not invalidate source review. Record the new merge SHA and tree equivalence after squash; any source drift needs renewed applicable review and checks.

## Release gates stay in place

This policy changes review scheduling and provider availability only. Required `rust (ubuntu-latest)`, `rust (macos-latest)`, `ui`, and other applicable checks still apply to the exact candidate. No branch-protection check is removed or bypassed. Never use a reviewer approval to conceal a failing required check.

Before a release tag, verify the exact merged source and required checks. Homebrew formula bumps, signed DMG / tarball assets, and finalize-release smokes remain separate from the scoped merge verdict.

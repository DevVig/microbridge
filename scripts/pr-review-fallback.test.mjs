import assert from "node:assert/strict";
import test from "node:test";

import {
  DEFAULTS,
  buildFallbackComment,
  classifyCheckOrStatus,
  classifyCodeRabbitText,
  decidePrFallback,
  extractReviewedHeadSha,
  formatMissingCodexTokenNotice,
  hasFallbackMarker,
  isDependencyBot,
  isExplicitCodeRabbitReviewRequest,
  readyAtMs,
  readConfig,
  selectFallbackBatch,
  shouldHandleIssueComment,
} from "../.github/pr-review-fallback/pr-review-fallback.mjs";

const RATE_LIMIT_COMMENT = `<!-- This is an auto-generated comment: summarize by coderabbit.ai -->
<!-- This is an auto-generated comment: rate limited by coderabbit.ai -->

> [!WARNING]
> ## Review limit reached
>
> Enable **usage-based reviews** in Billing to review now.
> **Next included review available in 58 seconds.**
`;

const FAILURE_COMMENT = `<!-- This is an auto-generated comment: summarize by coderabbit.ai -->
<!-- This is an auto-generated comment: failure by coderabbit.ai -->

> [!CAUTION]
> ## Review failed
`;

const DRAFT_SKIP_COMMENT = `<!-- This is an auto-generated comment: skip review by coderabbit.ai -->

> [!IMPORTANT]
> ## Draft PR not reviewed
`;

const COMPLETED_REVIEW_BODY = `**Actionable comments posted: 1**

<!-- This is an auto-generated comment by CodeRabbit for review status -->
`;

const WALKTHROUGH_COMMENT = `<!-- This is an auto-generated comment: summarize by coderabbit.ai -->
<!-- walkthrough_start -->
**Change:** Bug fix
<!-- final_review_risk_coverage:{"sourceCommitId":"083903d896e5ef55a97e5ddcf1ed2bf98530e7d2","coveredCommitId":"083903d896e5ef55a97e5ddcf1ed2bf98530e7d2","kind":"reviewed"} -->
`;

function pull({
  number = 579,
  draft = false,
  login = "DevVig",
  sha = "67ec5b01a1b9e249b2b1fd0acaf19097cdbf137c",
  created = "2026-10-05T15:31:06Z",
  title = "Example",
} = {}) {
  return {
    number,
    draft,
    title,
    created_at: created,
    user: { login },
    head: { sha },
  };
}

function comment({
  login = "coderabbitai[bot]",
  body = RATE_LIMIT_COMMENT,
  created = "2026-10-05T15:32:39Z",
} = {}) {
  return { user: { login }, body, created_at: created };
}

test("classifies this repo's CodeRabbit rate-limit HTML", () => {
  assert.equal(classifyCodeRabbitText(RATE_LIMIT_COMMENT), "rate_limit");
});

test("classifies this repo's CodeRabbit failure HTML", () => {
  assert.equal(classifyCodeRabbitText(FAILURE_COMMENT), "error");
});

test("classifies draft-skip comments as skip_draft, not completed", () => {
  assert.equal(classifyCodeRabbitText(DRAFT_SKIP_COMMENT), "skip_draft");
});

test("keeps a completed review when CodeRabbit later adds skip-review to the same comment", () => {
  const body = `${DRAFT_SKIP_COMMENT}\n${WALKTHROUGH_COMMENT}`;
  assert.equal(classifyCodeRabbitText(body), "completed");
});

test("classifies a completed CodeRabbit review body", () => {
  assert.equal(classifyCodeRabbitText(COMPLETED_REVIEW_BODY), "completed");
  assert.equal(classifyCodeRabbitText(WALKTHROUGH_COMMENT), "completed");
});

test("extracts coveredCommitId from a walkthrough comment", () => {
  assert.equal(
    extractReviewedHeadSha(WALKTHROUGH_COMMENT),
    "083903d896e5ef55a97e5ddcf1ed2bf98530e7d2",
  );
});

test("waits when a later CodeRabbit check is still running", () => {
  const decision = decidePrFallback({
    pull: pull(),
    comments: [comment({ created: "2026-10-05T15:32:39Z" })],
    checks: [
      {
        name: "CodeRabbit",
        status: "in_progress",
        started_at: "2026-10-05T16:00:00Z",
      },
    ],
    nowMs: Date.parse("2026-10-05T16:05:00Z"),
    windowMs: 45 * 60 * 1000,
  });
  assert.equal(decision.action, "skip");
  assert.equal(decision.reason, "coderabbit_in_progress");
});

test("classifies CodeRabbit commit status used on this repo", () => {
  assert.equal(
    classifyCheckOrStatus({
      context: "CodeRabbit",
      state: "success",
      description: "Review rate limited",
    }),
    "rate_limit",
  );
  assert.equal(
    classifyCheckOrStatus({
      name: "CodeRabbit",
      status: "in_progress",
    }),
    "in_progress",
  );
  assert.equal(
    classifyCheckOrStatus({
      name: "CodeRabbit",
      conclusion: "failure",
      output: { title: "Review crashed" },
    }),
    "error",
  );
});

test("detects dependabot and renovate authors", () => {
  assert.equal(isDependencyBot("dependabot[bot]"), true);
  assert.equal(isDependencyBot("renovate[bot]"), true);
  assert.equal(isDependencyBot("DevVig"), false);
});

test("skips drafts and dependency bots", () => {
  assert.equal(
    decidePrFallback({ pull: pull({ draft: true }) }).reason,
    "draft",
  );
  assert.equal(
    decidePrFallback({
      pull: pull({ login: "dependabot[bot]" }),
    }).reason,
    "dependency_bot",
  );
});

test("fires on a ready PR whose only CodeRabbit result is a rate limit", () => {
  const decision = decidePrFallback({
    pull: pull(),
    comments: [comment()],
    statuses: [
      {
        context: "CodeRabbit",
        state: "success",
        description: "Review rate limited",
        created_at: "2026-10-05T15:32:40Z",
      },
    ],
    nowMs: Date.parse("2026-10-06T00:00:00Z"),
    windowMs: 45 * 60 * 1000,
  });
  assert.equal(decision.action, "fallback");
  assert.equal(decision.reason, "rate_limit");
});

test("fires on a CodeRabbit failure comment", () => {
  const decision = decidePrFallback({
    pull: pull({ number: 302 }),
    comments: [comment({ body: FAILURE_COMMENT })],
    nowMs: Date.parse("2026-10-06T00:00:00Z"),
    windowMs: 45 * 60 * 1000,
  });
  assert.equal(decision.action, "fallback");
  assert.equal(decision.reason, "error");
});

test("does nothing when CodeRabbit reviewed the current head", () => {
  const sha = "083903d896e5ef55a97e5ddcf1ed2bf98530e7d2";
  const decision = decidePrFallback({
    pull: pull({ sha }),
    reviews: [
      {
        user: { login: "coderabbitai[bot]" },
        body: COMPLETED_REVIEW_BODY,
        commit_id: sha,
        submitted_at: "2026-10-03T19:57:03Z",
      },
    ],
    nowMs: Date.parse("2026-10-06T00:00:00Z"),
    windowMs: 45 * 60 * 1000,
  });
  assert.equal(decision.action, "skip");
  assert.equal(decision.reason, "coderabbit_reviewed_current_head");
});

test("does not wait after CodeRabbit appends skip-review to a completed walkthrough", () => {
  const laterHead = "4b786bebaa7a8b9bd95cc11c34d607f2d3150ee7";
  const decision = decidePrFallback({
    pull: pull({ number: 582, sha: laterHead }),
    comments: [
      comment({
        body: `${DRAFT_SKIP_COMMENT}\n${WALKTHROUGH_COMMENT}`,
        created: "2026-10-06T00:28:32Z",
      }),
    ],
    nowMs: Date.parse("2026-10-06T00:50:00Z"),
    windowMs: 45 * 60 * 1000,
  });
  assert.equal(decision.action, "skip");
  assert.equal(decision.reason, "coderabbit_reviewed_earlier_head");
});

test("does not misfire after later pushes when incremental review is off", () => {
  const reviewed = "083903d896e5ef55a97e5ddcf1ed2bf98530e7d2";
  const laterHead = "7ccd6c5ed5d8a15acc0e419d74143989fedeec84";
  const decision = decidePrFallback({
    pull: pull({ number: 551, sha: laterHead }),
    comments: [comment({ body: WALKTHROUGH_COMMENT, created: "2026-09-30T05:58:04Z" })],
    reviews: [
      {
        user: { login: "coderabbitai[bot]" },
        body: COMPLETED_REVIEW_BODY,
        commit_id: reviewed,
        submitted_at: "2026-10-03T19:57:03Z",
      },
    ],
    nowMs: Date.parse("2026-10-06T00:00:00Z"),
    windowMs: 45 * 60 * 1000,
  });
  assert.equal(decision.action, "skip");
  assert.equal(decision.reason, "coderabbit_reviewed_earlier_head");
});

test("window is relative to ready_for_review, not the latest push", () => {
  const created = "2026-10-05T15:31:06Z";
  const ready = "2026-10-05T22:42:53Z";
  const now = Date.parse("2026-10-05T23:10:00Z");
  const decision = decidePrFallback({
    pull: pull({ created }),
    timeline: [{ event: "ready_for_review", created_at: ready }],
    nowMs: now,
    windowMs: 45 * 60 * 1000,
  });
  assert.equal(decision.action, "skip");
  assert.equal(decision.reason, "waiting_for_coderabbit");
});

test("fires no_review_within_window after the PR has been ready long enough", () => {
  const ready = "2026-10-05T22:42:53Z";
  const decision = decidePrFallback({
    pull: pull({ created: "2026-10-05T15:31:06Z" }),
    timeline: [{ event: "ready_for_review", created_at: ready }],
    nowMs: Date.parse("2026-10-05T23:40:00Z"),
    windowMs: 45 * 60 * 1000,
  });
  assert.equal(decision.action, "fallback");
  assert.equal(decision.reason, "no_review_within_window");
});

test("an explicit @coderabbitai review after an earlier review reopens the window", () => {
  const reviewed = "083903d896e5ef55a97e5ddcf1ed2bf98530e7d2";
  const laterHead = "7ccd6c5ed5d8a15acc0e419d74143989fedeec84";
  const decision = decidePrFallback({
    pull: pull({ sha: laterHead }),
    reviews: [
      {
        user: { login: "coderabbitai[bot]" },
        body: COMPLETED_REVIEW_BODY,
        commit_id: reviewed,
        submitted_at: "2026-10-03T19:57:03Z",
      },
    ],
    comments: [
      comment({
        login: "DevVig",
        body: "@coderabbitai review",
        created: "2026-10-04T03:00:00Z",
      }),
    ],
    nowMs: Date.parse("2026-10-04T04:00:00Z"),
    windowMs: 45 * 60 * 1000,
  });
  assert.equal(decision.action, "fallback");
  assert.equal(decision.reason, "no_review_within_window");
});

test("a rate-limit after an explicit re-review request is a fallback", () => {
  const reviewed = "083903d896e5ef55a97e5ddcf1ed2bf98530e7d2";
  const laterHead = "7ccd6c5ed5d8a15acc0e419d74143989fedeec84";
  const decision = decidePrFallback({
    pull: pull({ sha: laterHead }),
    reviews: [
      {
        user: { login: "coderabbitai[bot]" },
        body: COMPLETED_REVIEW_BODY,
        commit_id: reviewed,
        submitted_at: "2026-10-03T19:57:03Z",
      },
    ],
    comments: [
      comment({
        login: "DevVig",
        body: "@coderabbitai full review",
        created: "2026-10-04T03:00:00Z",
      }),
      comment({
        body: RATE_LIMIT_COMMENT,
        created: "2026-10-04T03:01:00Z",
      }),
    ],
    nowMs: Date.parse("2026-10-04T03:02:00Z"),
    windowMs: 45 * 60 * 1000,
  });
  assert.equal(decision.action, "fallback");
  assert.equal(decision.reason, "rate_limit");
});

test("fires at most once per head SHA when a marker comment exists", () => {
  const sha = "67ec5b01a1b9e249b2b1fd0acaf19097cdbf137c";
  const decision = decidePrFallback({
    pull: pull({ sha }),
    comments: [
      comment(),
      comment({
        login: "github-actions[bot]",
        body: buildFallbackComment({
          headSha: sha,
          reason: "rate_limit",
          detail: "already requested",
        }),
      }),
    ],
    nowMs: Date.parse("2026-10-06T00:00:00Z"),
  });
  assert.equal(decision.action, "skip");
  assert.equal(decision.reason, "already_recorded");
});

test("selectFallbackBatch respects the per-run cap and keeps oldest-first order", () => {
  const decisions = [
    { action: "skip", number: 1, reason: "draft" },
    { action: "fallback", number: 579, reason: "rate_limit", readyAt: 3 },
    { action: "fallback", number: 503, reason: "rate_limit", readyAt: 1 },
    { action: "fallback", number: 600, reason: "no_review_within_window", readyAt: 2 },
  ];
  const batch = selectFallbackBatch(decisions, 2);
  assert.deepEqual(
    batch.selected.map((item) => item.number),
    [503, 600],
  );
  assert.deepEqual(
    batch.throttled.map((item) => item.number),
    [579],
  );
});

test("issue_comment handler only continues for CodeRabbit rate-limit or error", () => {
  assert.equal(
    shouldHandleIssueComment({
      eventName: "issue_comment",
      commentLogin: "vercel[bot]",
      commentBody: "preview",
    }),
    false,
  );
  assert.equal(
    shouldHandleIssueComment({
      eventName: "issue_comment",
      commentLogin: "coderabbitai[bot]",
      commentBody: RATE_LIMIT_COMMENT,
    }),
    true,
  );
  assert.equal(
    shouldHandleIssueComment({
      eventName: "schedule",
      commentLogin: "",
      commentBody: "",
    }),
    true,
  );
});

test("readyAtMs uses the latest ready_for_review event", () => {
  const ms = readyAtMs(pull({ created: "2026-10-03T19:00:00Z" }), [
    { event: "ready_for_review", created_at: "2026-10-03T19:25:02Z" },
    { event: "convert_to_draft", created_at: "2026-10-03T19:30:56Z" },
    { event: "ready_for_review", created_at: "2026-10-03T19:50:59Z" },
  ]);
  assert.equal(ms, Date.parse("2026-10-03T19:50:59Z"));
});

test("fallback comment records reason, SHA, and Codex mention without claiming a CodeRabbit pass", () => {
  const sha = "67ec5b01a1b9e249b2b1fd0acaf19097cdbf137c";
  const body = buildFallbackComment({
    headSha: sha,
    reason: "rate_limit",
    detail: "CodeRabbit reported a rate limit.",
    windowMinutes: 45,
  });
  assert.equal(hasFallbackMarker(body, sha), true);
  assert.match(body, /@codex review/);
  assert.match(body, /not a CodeRabbit pass/);
  assert.match(body, /rate limit/);
  assert.equal(isExplicitCodeRabbitReviewRequest({ user: { login: "bot" }, body }), false);
});

test("readConfig honors workflow defaults from env", () => {
  const config = readConfig({
    FALLBACK_WINDOW_MINUTES: "30",
    FALLBACK_MAX_PRS_PER_RUN: "1",
    FALLBACK_DRY_RUN: "true",
    GITHUB_REPOSITORY: "acme/widgets",
  });
  assert.equal(config.windowMinutes, 30);
  assert.equal(config.maxPrsPerRun, 1);
  assert.equal(config.dryRun, true);
  assert.equal(config.label, DEFAULTS.label);
  assert.equal(config.prNumber, null);
});

test("readConfig scopes evaluation to FALLBACK_PR_NUMBER when set", () => {
  const scoped = readConfig({
    FALLBACK_PR_NUMBER: "582",
    GITHUB_REPOSITORY: "acme/widgets",
  });
  assert.equal(scoped.prNumber, 582);
  const sweep = readConfig({
    FALLBACK_PR_NUMBER: "",
    GITHUB_REPOSITORY: "acme/widgets",
  });
  assert.equal(sweep.prNumber, null);
});

test("missing CODEX_GITHUB_TOKEN is a quiet warning, not a job failure", () => {
  const notice = formatMissingCodexTokenNotice([
    {
      number: 503,
      reason: "rate_limit",
      detail: "CodeRabbit reported a rate limit and has not completed a review of this head.",
    },
    {
      number: 579,
      reason: "rate_limit",
      detail: "CodeRabbit reported a rate limit and has not completed a review of this head.",
    },
  ]);
  assert.match(notice.annotation, /^::warning::CODEX_GITHUB_TOKEN is unset/);
  assert.match(notice.annotation, /Posted nothing/);
  assert.match(notice.annotation, /#503 rate_limit/);
  assert.match(notice.annotation, /#579 rate_limit/);
  assert.match(notice.summary, /Would have requested fallback for:/);
  assert.match(notice.summary, /#503 rate_limit:/);
  assert.match(notice.summary, /#579 rate_limit:/);
  assert.doesNotMatch(notice.annotation, /exit 1|throw|fatal/i);
});

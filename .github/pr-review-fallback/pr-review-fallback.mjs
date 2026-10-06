#!/usr/bin/env node
/**
 * Portable CodeRabbit → Codex fallback reviewer.
 *
 * Request a Codex review only when CodeRabbit cannot review a ready PR.
 * Copy this directory and the sibling workflow into other repos as-is.
 * Override values at the top of `.github/workflows/coderabbit-fallback-review.yml`
 * or via the environment variables listed in DEFAULTS.
 *
 * Codex trigger (verified 2026-10-06 against https://learn.chatgpt.com/docs/third-party/github):
 * post a top-level PR comment whose text includes `@codex review`.
 * Codex maps the comment author to a ChatGPT-connected GitHub account, so
 * `github-actions[bot]` / GITHUB_TOKEN mentions are not a reliable trigger.
 * Set repo secret CODEX_GITHUB_TOKEN to a PAT for the connected user.
 * If that secret is missing, skip posting, emit a warning, and exit 0.
 */

import { appendFileSync } from "node:fs";

export const DEFAULTS = {
  windowMinutes: 45,
  maxPrsPerRun: 2,
  label: "review-fallback-coderabbit-unavailable",
  coderabbitLogins: ["coderabbitai[bot]", "coderabbitai"],
  dependencyBotLogins: [
    "dependabot[bot]",
    "dependabot",
    "renovate[bot]",
    "renovate",
  ],
  markerName: "pr-review-fallback",
  markerVersion: "v1",
  codexTrigger: "@codex review",
  githubApiVersion: "2022-11-28",
};

const RATE_LIMIT_MARKERS = [
  /rate limited by coderabbit\.ai/i,
  /## Review limit reached/i,
  /\bReview rate limited\b/i,
  /\bReview rate limit exceeded\b/i,
  /\byou've reached your PR review limit\b/i,
  /\bRate limit exceeded\b/i,
];

const ERROR_MARKERS = [
  /failure by coderabbit\.ai/i,
  /## Review failed/i,
  /failed to (start|complete) (the )?review/i,
  /unable to (start|complete) (the )?review/i,
  /encountered an error (while|during) (this )?review/i,
];

const DRAFT_SKIP_MARKERS = [
  /skip review by coderabbit\.ai/i,
  /## Draft PR not reviewed/i,
];

const COMPLETED_REVIEW_MARKERS = [
  /<!-- This is an auto-generated comment by CodeRabbit for review status -->/,
  /Actionable comments posted:/i,
  /<!-- walkthrough_start -->/,
  /final_review_risk_coverage/,
];

const EXPLICIT_REVIEW_REQUEST =
  /@coderabbitai\s+(?:full\s+)?review\b/i;

const COMMIT_SHA = /\b[0-9a-f]{40}\b/gi;
const COVERED_COMMIT = /"coveredCommitId"\s*:\s*"([0-9a-f]{40})"/i;
const BETWEEN_COMMITS =
  /between\s+[0-9a-f]{7,40}\s+and\s+([0-9a-f]{7,40})/i;

export function readConfig(env = process.env) {
  const windowMinutes = positiveInt(
    env.FALLBACK_WINDOW_MINUTES,
    DEFAULTS.windowMinutes,
  );
  const maxPrsPerRun = positiveInt(
    env.FALLBACK_MAX_PRS_PER_RUN,
    DEFAULTS.maxPrsPerRun,
  );
  return {
    windowMinutes,
    windowMs: windowMinutes * 60 * 1000,
    maxPrsPerRun,
    label: env.FALLBACK_LABEL || DEFAULTS.label,
    coderabbitLogins: parseLoginList(
      env.FALLBACK_CODERABBIT_LOGINS,
      DEFAULTS.coderabbitLogins,
    ),
    dependencyBotLogins: parseLoginList(
      env.FALLBACK_DEPENDENCY_BOT_LOGINS,
      DEFAULTS.dependencyBotLogins,
    ),
    markerName: DEFAULTS.markerName,
    markerVersion: DEFAULTS.markerVersion,
    codexTrigger: env.FALLBACK_CODEX_TRIGGER || DEFAULTS.codexTrigger,
    ownerRepo: env.GITHUB_REPOSITORY || "",
    commentToken: env.CODEX_GITHUB_TOKEN || "",
    apiToken: env.GITHUB_TOKEN || env.CODEX_GITHUB_TOKEN || "",
    apiBase: (env.GITHUB_API_URL || "https://api.github.com").replace(/\/$/, ""),
    dryRun: parseBoolean(env.FALLBACK_DRY_RUN, false),
    prNumber: env.FALLBACK_PR_NUMBER ? Number(env.FALLBACK_PR_NUMBER) : null,
    eventName: env.GITHUB_EVENT_NAME || "",
    commentLogin: env.FALLBACK_COMMENT_LOGIN || "",
    commentBody: env.FALLBACK_COMMENT_BODY || "",
    nowMs: env.FALLBACK_NOW_MS ? Number(env.FALLBACK_NOW_MS) : Date.now(),
  };
}

function positiveInt(value, fallback) {
  const parsed = Number.parseInt(String(value ?? ""), 10);
  return Number.isSafeInteger(parsed) && parsed > 0 ? parsed : fallback;
}

function parseBoolean(value, fallback) {
  if (value == null || value === "") return fallback;
  const normalized = String(value).trim().toLowerCase();
  if (["1", "true", "yes", "on"].includes(normalized)) return true;
  if (["0", "false", "no", "off"].includes(normalized)) return false;
  return fallback;
}

function parseLoginList(value, fallback) {
  if (!value) return fallback;
  const items = String(value)
    .split(",")
    .map((item) => item.trim())
    .filter(Boolean);
  return items.length > 0 ? items : fallback;
}

function loginSet(logins) {
  return new Set(logins.map((login) => login.toLowerCase()));
}

export function isDependencyBot(login, config = DEFAULTS) {
  if (!login) return false;
  return loginSet(config.dependencyBotLogins).has(login.toLowerCase());
}

export function isCodeRabbitLogin(login, config = DEFAULTS) {
  if (!login) return false;
  return loginSet(config.coderabbitLogins).has(login.toLowerCase());
}

export function classifyCodeRabbitText(text) {
  const body = text || "";
  if (RATE_LIMIT_MARKERS.some((pattern) => pattern.test(body))) {
    return "rate_limit";
  }
  if (ERROR_MARKERS.some((pattern) => pattern.test(body))) {
    return "error";
  }
  // CodeRabbit reuses one summary comment. After a completed review it may
  // later add `skip review` (incremental off). Treat that as completed.
  if (COMPLETED_REVIEW_MARKERS.some((pattern) => pattern.test(body))) {
    return "completed";
  }
  if (DRAFT_SKIP_MARKERS.some((pattern) => pattern.test(body))) {
    return "skip_draft";
  }
  return "other";
}

export function extractReviewedHeadSha(text) {
  const body = text || "";
  const covered = body.match(COVERED_COMMIT);
  if (covered?.[1]) return covered[1];
  const between = body.match(BETWEEN_COMMITS);
  if (between?.[1] && between[1].length === 40) return between[1];
  const shas = body.match(COMMIT_SHA) || [];
  return shas.length > 0 ? shas[shas.length - 1] : null;
}

export function isExplicitCodeRabbitReviewRequest(comment, config = DEFAULTS) {
  if (!comment?.body || isCodeRabbitLogin(comment.user?.login, config)) {
    return false;
  }
  if (hasFallbackMarker(comment.body, null, config)) return false;
  return EXPLICIT_REVIEW_REQUEST.test(comment.body);
}

export function fallbackMarkerHtml(headSha, reason, config = DEFAULTS) {
  const sha = headSha || "";
  return [
    `<!-- ${config.markerName}:${config.markerVersion} -->`,
    `<!-- ${config.markerName}:sha:${sha} -->`,
    `<!-- ${config.markerName}:reason:${reason} -->`,
  ].join("\n");
}

export function hasFallbackMarker(body, headSha, config = DEFAULTS) {
  const text = body || "";
  if (headSha) {
    return text.includes(`<!-- ${config.markerName}:sha:${headSha} -->`);
  }
  return text.includes(`<!-- ${config.markerName}:${config.markerVersion} -->`);
}

export function classifyCheckOrStatus(item) {
  const name = `${item.name || item.context || ""}`;
  const description = `${item.description || item.title || item.output?.title || ""}`;
  const combined = `${name}\n${description}\n${item.output?.summary || ""}`;
  const status = item.status || "";
  const conclusion = item.conclusion || item.state || "";

  if (/coderabbit|review rate limited/i.test(name) && /queued|in_progress/i.test(status)) {
    return "in_progress";
  }
  if (classifyCodeRabbitText(combined) === "rate_limit") return "rate_limit";
  if (/coderabbit/i.test(name) && /failure|error/i.test(conclusion)) {
    return "error";
  }
  if (classifyCodeRabbitText(combined) === "error") return "error";
  return "other";
}

function eventTime(item) {
  const raw =
    item.submitted_at ||
    item.updated_at ||
    item.created_at ||
    item.completed_at ||
    item.started_at ||
    0;
  const ms = Date.parse(raw);
  return Number.isFinite(ms) ? ms : 0;
}

export function readyAtMs(pull, timelineEvents = []) {
  const readyEvents = timelineEvents
    .filter((event) => event.event === "ready_for_review")
    .map((event) => Date.parse(event.created_at))
    .filter((ms) => Number.isFinite(ms));
  if (readyEvents.length > 0) return Math.max(...readyEvents);
  return Date.parse(pull.created_at) || 0;
}

function collectSignals({ comments, reviews, checks, statuses }, config) {
  const signals = [];

  for (const comment of comments) {
    if (!isCodeRabbitLogin(comment.user?.login, config)) continue;
    const kind = classifyCodeRabbitText(comment.body);
    if (kind === "other") continue;
    signals.push({
      kind,
      at: eventTime(comment),
      headSha: extractReviewedHeadSha(comment.body),
      source: "comment",
    });
  }

  for (const review of reviews) {
    if (!isCodeRabbitLogin(review.user?.login, config)) continue;
    const kind = classifyCodeRabbitText(review.body);
    if (kind !== "completed" && kind !== "rate_limit" && kind !== "error") {
      continue;
    }
    signals.push({
      kind,
      at: eventTime(review),
      headSha: review.commit_id || extractReviewedHeadSha(review.body),
      source: "review",
    });
  }

  for (const item of [...checks, ...statuses]) {
    const kind = classifyCheckOrStatus(item);
    if (kind === "other") continue;
    signals.push({
      kind,
      at: eventTime(item),
      headSha: item.head_sha || null,
      source: "check",
    });
  }

  return signals.sort((left, right) => left.at - right.at);
}

function sameSha(left, right) {
  if (!left || !right) return false;
  const a = left.toLowerCase();
  const b = right.toLowerCase();
  if (a === b) return true;
  return a.startsWith(b) || b.startsWith(a);
}

export function decidePrFallback(input, config = DEFAULTS) {
  const {
    pull,
    comments = [],
    reviews = [],
    checks = [],
    statuses = [],
    timeline = [],
    nowMs = Date.now(),
    windowMs = (config.windowMinutes || DEFAULTS.windowMinutes) * 60 * 1000,
  } = input;

  const number = pull.number;
  const headSha = pull.head?.sha || "";
  const title = pull.title || "";

  const readyAt = readyAtMs(pull, timeline);

  if (pull.draft) {
    return skip(number, headSha, title, "draft", "Draft pull requests are skipped.", readyAt);
  }

  if (isDependencyBot(pull.user?.login, config)) {
    return skip(
      number,
      headSha,
      title,
      "dependency_bot",
      `Author ${pull.user.login} is a dependency bot.`,
      readyAt,
    );
  }

  if (comments.some((comment) => hasFallbackMarker(comment.body, headSha, config))) {
    return skip(
      number,
      headSha,
      title,
      "already_recorded",
      "A fallback marker already exists for this head SHA.",
      readyAt,
    );
  }

  const signals = collectSignals({ comments, reviews, checks, statuses }, config);
  const completed = signals.filter((signal) => signal.kind === "completed");
  const completedOnHead = completed.find((signal) => sameSha(signal.headSha, headSha));
  if (completedOnHead) {
    return skip(
      number,
      headSha,
      title,
      "coderabbit_reviewed_current_head",
      "CodeRabbit completed a review that covers the current head.",
      readyAt,
    );
  }

  const lastCompleted = completed.at(-1) || null;
  const explicitRequests = comments
    .filter((comment) => isExplicitCodeRabbitReviewRequest(comment, config))
    .map((comment) => ({ at: eventTime(comment), login: comment.user?.login }))
    .sort((left, right) => left.at - right.at);
  const requestAfterCompleted = explicitRequests.filter(
    (request) => !lastCompleted || request.at > lastCompleted.at,
  );
  const lastRequest = requestAfterCompleted.at(-1) || null;

  if (lastCompleted && !lastRequest) {
    return skip(
      number,
      headSha,
      title,
      "coderabbit_reviewed_earlier_head",
      "CodeRabbit already reviewed an earlier head and incremental review is off; no explicit @coderabbitai review was requested after that review.",
      readyAt,
    );
  }

  const latestInProgress = [...signals]
    .reverse()
    .find((signal) => signal.kind === "in_progress");
  const latestBlocking = [...signals]
    .reverse()
    .find((signal) => signal.kind === "rate_limit" || signal.kind === "error");
  if (
    latestInProgress &&
    (!latestBlocking || latestInProgress.at >= latestBlocking.at)
  ) {
    return skip(
      number,
      headSha,
      title,
      "coderabbit_in_progress",
      "A CodeRabbit check is still in progress.",
      readyAt,
    );
  }
  const blockingAfterObligation =
    latestBlocking &&
    (!lastCompleted || latestBlocking.at >= lastCompleted.at) &&
    (!lastRequest || latestBlocking.at >= lastRequest.at);

  if (blockingAfterObligation && latestBlocking.kind === "rate_limit") {
    return fallback(
      number,
      headSha,
      title,
      "rate_limit",
      "CodeRabbit reported a rate limit and has not completed a review of this head.",
      readyAt,
    );
  }
  if (blockingAfterObligation && latestBlocking.kind === "error") {
    return fallback(
      number,
      headSha,
      title,
      "error",
      "CodeRabbit reported a review error and has not completed a review of this head.",
      readyAt,
    );
  }

  const obligationAt = lastRequest?.at || readyAt;
  const elapsedMs = nowMs - obligationAt;
  if (obligationAt > 0 && elapsedMs >= windowMs) {
    return fallback(
      number,
      headSha,
      title,
      "no_review_within_window",
      lastRequest
        ? `No CodeRabbit review completed within ${Math.round(windowMs / 60000)} minutes of the explicit review request.`
        : `No CodeRabbit review completed within ${Math.round(windowMs / 60000)} minutes of the PR becoming ready.`,
      readyAt,
    );
  }

  return skip(
    number,
    headSha,
    title,
    "waiting_for_coderabbit",
    `Waiting for CodeRabbit; ${Math.max(0, windowMs - elapsedMs)} ms remain in the window.`,
    readyAt,
  );
}

function skip(number, headSha, title, reason, detail, readyAt = 0) {
  return {
    action: "skip",
    number,
    headSha,
    title,
    reason,
    detail,
    readyAt,
  };
}

function fallback(number, headSha, title, reason, detail, readyAt = 0) {
  return {
    action: "fallback",
    number,
    headSha,
    title,
    reason,
    detail,
    readyAt,
  };
}

export function selectFallbackBatch(decisions, maxPrsPerRun = DEFAULTS.maxPrsPerRun) {
  const eligible = decisions
    .filter((decision) => decision.action === "fallback")
    .slice()
    .sort((left, right) => (left.readyAt || left.number) - (right.readyAt || right.number));
  return {
    eligible,
    selected: eligible.slice(0, maxPrsPerRun),
    throttled: eligible.slice(maxPrsPerRun),
  };
}

export function buildFallbackComment({
  headSha,
  reason,
  detail,
  windowMinutes = DEFAULTS.windowMinutes,
  config = DEFAULTS,
  mentionCodex = true,
}) {
  const reasonLabel = {
    rate_limit: "rate limit",
    error: "error / failed to start or complete",
    no_review_within_window: `no review within ${windowMinutes} min`,
  }[reason] || reason;

  const lines = [];
  if (mentionCodex) lines.push(config.codexTrigger, "");
  lines.push(
    fallbackMarkerHtml(headSha, reason, config),
    "",
    "## Fallback review (not a CodeRabbit pass)",
    "",
    `CodeRabbit was unavailable for head \`${headSha}\`.`,
    `Reason: ${reasonLabel}.`,
    detail,
    "",
    mentionCodex
      ? "Codex was requested as the independent fallback. This is not a CodeRabbit review and does not count as a CodeRabbit pass."
      : "Codex was **not** mentioned because `CODEX_GITHUB_TOKEN` is unset. A PAT from the GitHub user connected to Codex is required; `github-actions[bot]` mentions are not a reliable Codex trigger.",
    "",
    "See `docs/ops/pr-review-policy.md`.",
  );
  return lines.join("\n");
}

export function formatMissingCodexTokenNotice(selected) {
  const listing = selected
    .map((item) => {
      const why = item.detail ? `${item.reason}: ${item.detail}` : item.reason;
      return `#${item.number} ${why}`;
    })
    .join("; ");
  return {
    annotation: `::warning::CODEX_GITHUB_TOKEN is unset. Would have requested Codex fallback for: ${listing}. Posted nothing.`,
    summary: [
      "### Codex fallback skipped: `CODEX_GITHUB_TOKEN` unset",
      "",
      "Posted nothing. Add a PAT for the GitHub user whose ChatGPT account has this repository connected.",
      "",
      "Would have requested fallback for:",
      ...selected.map((item) => {
        const why = item.detail ? `${item.reason}: ${item.detail}` : item.reason;
        return `- #${item.number} ${why}`;
      }),
    ].join("\n"),
  };
}

function emitMissingCodexTokenNotice(selected, env = process.env) {
  const notice = formatMissingCodexTokenNotice(selected);
  console.log(notice.annotation);
  if (env.GITHUB_STEP_SUMMARY) {
    appendFileSync(env.GITHUB_STEP_SUMMARY, `${notice.summary}\n`);
  }
  return notice;
}

export function shouldHandleIssueComment({ eventName, commentLogin, commentBody, config = DEFAULTS }) {
  if (eventName !== "issue_comment") return true;
  if (!isCodeRabbitLogin(commentLogin, config)) return false;
  const kind = classifyCodeRabbitText(commentBody);
  return kind === "rate_limit" || kind === "error";
}

async function githubRequest(config, method, path, body) {
  const response = await fetch(`${config.apiBase}${path}`, {
    method,
    headers: {
      Accept: "application/vnd.github+json",
      Authorization: `Bearer ${config.apiToken}`,
      "X-GitHub-Api-Version": DEFAULTS.githubApiVersion,
      ...(body ? { "Content-Type": "application/json" } : {}),
    },
    body: body ? JSON.stringify(body) : undefined,
  });
  if (!response.ok) {
    const text = await response.text();
    throw new Error(`${method} ${path} failed: ${response.status} ${text}`);
  }
  if (response.status === 204) return null;
  return await response.json();
}

async function githubPaginate(config, path) {
  const items = [];
  let next = path;
  while (next) {
    const response = await fetch(`${config.apiBase}${next}`, {
      headers: {
        Accept: "application/vnd.github+json",
        Authorization: `Bearer ${config.apiToken}`,
        "X-GitHub-Api-Version": DEFAULTS.githubApiVersion,
      },
    });
    if (!response.ok) {
      const text = await response.text();
      throw new Error(`GET ${next} failed: ${response.status} ${text}`);
    }
    const page = await response.json();
    items.push(...(Array.isArray(page) ? page : page.check_runs || page.statuses || []));
    const link = response.headers.get("link") || "";
    const nextUrl = link.match(/<([^>]+)>;\s*rel="next"/)?.[1];
    if (nextUrl) {
      const url = new URL(nextUrl);
      next = `${url.pathname}${url.search}`;
    } else {
      next = null;
    }
  }
  return items;
}

async function listOpenPulls(config) {
  if (config.prNumber) {
    const pull = await githubRequest(
      config,
      "GET",
      `/repos/${config.ownerRepo}/pulls/${config.prNumber}`,
    );
    return [pull];
  }
  return githubPaginate(config, `/repos/${config.ownerRepo}/pulls?state=open&per_page=100`);
}

async function loadPrEvidence(config, pull) {
  const [comments, reviews, timeline, checkPayload, statusPayload] = await Promise.all([
    githubPaginate(config, `/repos/${config.ownerRepo}/issues/${pull.number}/comments?per_page=100`),
    githubPaginate(config, `/repos/${config.ownerRepo}/pulls/${pull.number}/reviews?per_page=100`),
    githubPaginate(config, `/repos/${config.ownerRepo}/issues/${pull.number}/timeline?per_page=100`),
    githubRequest(
      config,
      "GET",
      `/repos/${config.ownerRepo}/commits/${pull.head.sha}/check-runs?per_page=100`,
    ).catch(() => ({ check_runs: [] })),
    githubRequest(
      config,
      "GET",
      `/repos/${config.ownerRepo}/commits/${pull.head.sha}/status`,
    ).catch(() => ({ statuses: [] })),
  ]);
  return {
    comments,
    reviews,
    timeline,
    checks: checkPayload.check_runs || [],
    statuses: statusPayload.statuses || [],
  };
}

async function ensureLabel(config) {
  try {
    await githubRequest(
      config,
      "GET",
      `/repos/${config.ownerRepo}/labels/${encodeURIComponent(config.label)}`,
    );
  } catch {
    await githubRequest(config, "POST", `/repos/${config.ownerRepo}/labels`, {
      name: config.label,
      color: "c2a337",
      description: "CodeRabbit was unavailable; Codex was requested as fallback review",
    });
  }
}

async function applyFallback(config, decision) {
  const mentionCodex = Boolean(config.commentToken);
  if (!mentionCodex) {
    throw new Error(
      "CODEX_GITHUB_TOKEN is required to mention @codex review. github-actions[bot] comments are not a reliable Codex trigger because Codex maps the comment author to a ChatGPT-connected GitHub account.",
    );
  }

  const commentTokenConfig = { ...config, apiToken: config.commentToken };
  await ensureLabel(commentTokenConfig);
  await githubRequest(
    commentTokenConfig,
    "POST",
    `/repos/${config.ownerRepo}/issues/${decision.number}/labels`,
    { labels: [config.label] },
  );
  await githubRequest(
    commentTokenConfig,
    "POST",
    `/repos/${config.ownerRepo}/issues/${decision.number}/comments`,
    {
      body: buildFallbackComment({
        headSha: decision.headSha,
        reason: decision.reason,
        detail: decision.detail,
        windowMinutes: config.windowMinutes,
        config,
        mentionCodex: true,
      }),
    },
  );
}

function printReport(decisions, batch, config) {
  const lines = [
    `Fallback window: ${config.windowMinutes} minutes`,
    `Throttle: ${config.maxPrsPerRun} PR(s) per run`,
    `Dry run: ${config.dryRun}`,
    "",
    "Decisions:",
  ];
  for (const decision of decisions) {
    lines.push(
      `- #${decision.number} ${decision.action} ${decision.reason} (${decision.headSha.slice(0, 12)}): ${decision.detail}`,
    );
  }
  if (batch.selected.length > 0) {
    lines.push("", config.dryRun ? "Would request fallback for:" : "Requesting fallback for:");
    for (const decision of batch.selected) {
      lines.push(`- #${decision.number} ${decision.reason}`);
    }
  }
  if (batch.throttled.length > 0) {
    lines.push("", "Throttled until a later run:");
    for (const decision of batch.throttled) {
      lines.push(`- #${decision.number} ${decision.reason}`);
    }
  }
  console.log(lines.join("\n"));
}

export async function runFallback(env = process.env) {
  const config = readConfig(env);
  if (!config.ownerRepo) throw new Error("GITHUB_REPOSITORY is required");
  if (!config.apiToken) throw new Error("GITHUB_TOKEN is required");

  if (
    !shouldHandleIssueComment({
      eventName: config.eventName,
      commentLogin: config.commentLogin,
      commentBody: config.commentBody,
      config,
    })
  ) {
    console.log("Ignoring issue_comment from a non-CodeRabbit failure/rate-limit comment.");
    return { decisions: [], selected: [], throttled: [] };
  }

  const pulls = (await listOpenPulls(config)).filter((pull) => !pull.draft);
  const decisions = [];
  for (const pull of pulls) {
    if (isDependencyBot(pull.user?.login, config)) {
      decisions.push(
        skip(
          pull.number,
          pull.head?.sha || "",
          pull.title || "",
          "dependency_bot",
          `Author ${pull.user.login} is a dependency bot.`,
        ),
      );
      continue;
    }
    const evidence = await loadPrEvidence(config, pull);
    decisions.push(
      decidePrFallback(
        {
          pull,
          ...evidence,
          nowMs: config.nowMs,
          windowMs: config.windowMs,
        },
        config,
      ),
    );
  }

  const batch = selectFallbackBatch(decisions, config.maxPrsPerRun);
  printReport(decisions, batch, config);

  if (config.dryRun || batch.selected.length === 0) {
    return { decisions, ...batch };
  }

  if (!config.commentToken) {
    emitMissingCodexTokenNotice(batch.selected, env);
    return { decisions, ...batch, skippedMissingToken: true };
  }

  for (const decision of batch.selected) {
    await applyFallback(config, decision);
    console.log(`Recorded fallback on #${decision.number} for ${decision.headSha}`);
  }
  return { decisions, ...batch };
}

const invokedDirectly =
  process.argv[1] &&
  (process.argv[1].endsWith("pr-review-fallback.mjs") ||
    process.argv[1].endsWith("pr-review-fallback.js"));

if (invokedDirectly && process.env.FALLBACK_SKIP_MAIN !== "1") {
  runFallback().catch((error) => {
    console.error(error instanceof Error ? error.message : error);
    process.exitCode = 1;
  });
}

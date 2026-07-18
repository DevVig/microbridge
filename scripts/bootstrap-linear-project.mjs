#!/usr/bin/env node
/**
 * Create VIGDEV Linear project + Phase A/B issues for Microbridge.
 * Usage: LINEAR_API_KEY=lin_api_... node scripts/bootstrap-linear-project.mjs
 */
const KEY = process.env.LINEAR_API_KEY;
if (!KEY) {
  console.error("Set LINEAR_API_KEY (https://linear.app/settings/api)");
  process.exit(1);
}

async function gql(query, variables) {
  const res = await fetch("https://api.linear.app/graphql", {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
      Authorization: KEY,
    },
    body: JSON.stringify({ query, variables }),
  });
  const json = await res.json();
  if (json.errors?.length) {
    throw new Error(JSON.stringify(json.errors, null, 2));
  }
  return json.data;
}

const PHASE_A = [
  {
    title: "Public repo + release hygiene (v0.1.0)",
    done: true,
    description: "Done: repo public, versions at 0.1.0, tagged from main.",
  },
  {
    title: "Prove signed release pipeline",
    done: true,
    description: "Done: v0.1.0 Release with notarized DMGs + daemon archives.",
  },
  {
    title: "Homebrew prebuilt formula",
    done: true,
    description: "Done: Formula installs release assets; bump-formula.sh fixed.",
  },
  {
    title: "Install path QA (stranger path)",
    done: true,
    description: "Done: public assets + Linux aarch64 gated; brew/DMG paths documented.",
  },
  {
    title: "OSS hardening (PRIVACY, SECURITY, socket 0600)",
    done: true,
    description: "Done: PRIVACY.md, SECURITY.md, socket/dir permissions.",
  },
  {
    title: "Native action pipeline (Codex/Claude approve/reject/interrupt)",
    done: false,
    description:
      "Replace log-only in-process route_action with real agent control paths (pre-HID via ctl).",
  },
  {
    title: "Focus policy cleanup + frontmost footprint note",
    done: true,
    description: "Done: auto-follow docs + 400ms poll documented as footprint exception.",
  },
  {
    title: "UI honesty pass (Simulator/Detected/Connected)",
    done: true,
    description: "Done: chip honesty + scaffold-only Cursor/T3 labels.",
  },
];

const PHASE_B = [
  {
    title: "HID capture checklist (VID/PID, report map)",
    description:
      "Blocked until Micro arrives 2026-07-22. See docs/device-hid.md.",
  },
  {
    title: "Real HidDevice claim + LED packing + input poll",
    description: "Blocked until Micro arrives 2026-07-22.",
  },
  {
    title: "Listen-mode remapping + exclusive-ownership UX",
    description: "Blocked until Micro arrives 2026-07-22.",
  },
  {
    title: "Hardware release v0.2.0 after bidirectional Micro path",
    description: "Ship after HID path proven on real device.",
  },
];

async function main() {
  const { teams, issueLabels } = await gql(`{
    teams { nodes { id key name } }
    issueLabels { nodes { id name } }
  }`);
  const team =
    teams.nodes.find((t) => t.key === "VIGDEV") ||
    teams.nodes.find((t) => /vig|dev/i.test(t.key + t.name));
  if (!team) {
    throw new Error(
      `VIGDEV team not found. Teams: ${teams.nodes.map((t) => t.key).join(", ")}`,
    );
  }
  console.log("team", team.key, team.id);

  let blockedLabel = issueLabels.nodes.find((l) => l.name === "Blocked Dependency");
  if (!blockedLabel) {
    const created = await gql(
      `mutation($input: IssueLabelCreateInput!) {
        issueLabelCreate(input: $input) { issueLabel { id name } }
      }`,
      {
        input: {
          name: "Blocked Dependency",
          color: "#B60205",
          description: "Waiting on external dependency (e.g. Micro hardware)",
          teamId: team.id,
        },
      },
    );
    blockedLabel = created.issueLabelCreate.issueLabel;
  }

  const projectCreate = await gql(
    `mutation($input: ProjectCreateInput!) {
      projectCreate(input: $input) {
        success
        project { id name url }
      }
    }`,
    {
      input: {
        name: "Microbridge Production Launch",
        description:
          "Software productionization (Phase A → 2026-07-21) then Micro HID (Phase B from 2026-07-22). Repo: https://github.com/DevVig/microbridge",
        teamIds: [team.id],
        startDate: "2026-07-17",
        targetDate: "2026-08-15",
      },
    },
  );
  const project = projectCreate.projectCreate.project;
  console.log("project", project.url || project.id);

  const states = await gql(
    `query($teamId: String!) {
      team(id: $teamId) {
        states { nodes { id name type } }
      }
    }`,
    { teamId: team.id },
  );
  const doneState = states.team.states.nodes.find(
    (s) => s.type === "completed" || /done|complete/i.test(s.name),
  );
  const backlogState = states.team.states.nodes.find(
    (s) => s.type === "unstarted" || /backlog|todo/i.test(s.name),
  );

  async function createIssue({ title, description, done, blocked }) {
    const input = {
      teamId: team.id,
      projectId: project.id,
      title,
      description,
    };
    if (done && doneState) input.stateId = doneState.id;
    else if (backlogState) input.stateId = backlogState.id;
    if (blocked && blockedLabel) input.labelIds = [blockedLabel.id];

    const data = await gql(
      `mutation($input: IssueCreateInput!) {
        issueCreate(input: $input) {
          issue { id identifier url title }
        }
      }`,
      { input },
    );
    const issue = data.issueCreate.issue;
    console.log(done ? "DONE" : blocked ? "BLOCKED" : "OPEN", issue.identifier, issue.title);
    return issue;
  }

  for (const item of PHASE_A) {
    await createIssue({ ...item, blocked: false });
  }
  for (const item of PHASE_B) {
    await createIssue({ ...item, done: false, blocked: true });
  }

  console.log("\nLinear project ready:", project.url || project.id);
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});

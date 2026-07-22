import { describe, expect, it } from "vitest";

import {
  integrationGuidance,
  setupNextStep,
} from "./integrationSetup";
import { openableHostApp } from "./openHostApp";

describe("setupNextStep", () => {
  it("returns host-specific checklist copy", () => {
    expect(setupNextStep("cursor")).toContain("Reload Cursor");
    expect(setupNextStep("opencode")).toContain("Restart OpenCode");
    expect(setupNextStep("factory")).toContain("Droid");
    expect(setupNextStep("t3code")).toContain("pairing link");
    expect(setupNextStep("synara")).toBeNull();
  });
});

describe("integrationGuidance", () => {
  it("guides Cursor through enable → reload → events", () => {
    const disabled = integrationGuidance("cursor", "disabled");
    expect(disabled?.primaryAction).toBe("enable");
    expect(disabled?.steps[0]).toContain("install");

    const setup = integrationGuidance("cursor", "needs_setup", {
      enabled: true,
    });
    expect(setup?.title).toContain("Cursor");
    expect(setup?.steps.some((step) => step.includes("Reload"))).toBe(true);
    expect(setup?.primaryAction).toBe("open_app");
  });

  it("treats connected Cursor as lifecycle success, not broken Limited", () => {
    const connected = integrationGuidance("cursor", "connected");
    expect(connected?.title).toContain("connected");
    expect(connected?.steps.some((step) => /ACP|lifecycle/i.test(step))).toBe(
      true,
    );
  });

  it("guides Cursor ACP separately from IDE Composer", () => {
    const acp = integrationGuidance("cursor_acp", "needs_setup", {
      enabled: true,
    });
    expect(acp?.steps.some((step) => step.includes("CLI"))).toBe(true);
  });

  it("treats auto-discovered needs_setup + disabled as install CTA", () => {
    const detected = integrationGuidance("cursor", "needs_setup", {
      enabled: false,
    });
    expect(detected?.title).toContain("Detected");
    expect(detected?.primaryAction).toBe("enable");
  });

  it("covers T3 pairing, CNVS start, and idle Synara", () => {
    const t3 = integrationGuidance("t3code", "needs_setup", { enabled: true });
    expect(t3?.primaryAction).toBe("pair");
    expect(t3?.steps.some((step) => step.includes("Network access"))).toBe(
      true,
    );

    const cnvs = integrationGuidance("cnvs", "needs_setup");
    expect(cnvs?.steps[0]).toContain("CNVS");

    const synara = integrationGuidance("synara", "connected", {
      label: "Idle",
    });
    expect(synara?.steps[0]).toContain("Ready/Idle is normal");
  });

  it("explains always-on Codex/Claude watchers", () => {
    const codex = integrationGuidance("codex", "connected");
    expect(codex?.steps[0]).toContain("always on");
  });
});

describe("openableHostApp", () => {
  it("names apps we can open from Integrations", () => {
    expect(openableHostApp("cursor")).toBe("Cursor");
    expect(openableHostApp("opencode")).toBe("OpenCode");
    expect(openableHostApp("t3code")).toBe("T3 Code");
    expect(openableHostApp("factory")).toBeNull();
  });
});

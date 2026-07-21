import { describe, expect, it } from "vitest";

import { setupNextStep } from "./integrationSetup";
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

describe("openableHostApp", () => {
  it("names apps we can open from Integrations", () => {
    expect(openableHostApp("cursor")).toBe("Cursor");
    expect(openableHostApp("opencode")).toBe("OpenCode");
    expect(openableHostApp("t3code")).toBe("T3 Code");
    expect(openableHostApp("factory")).toBeNull();
  });
});

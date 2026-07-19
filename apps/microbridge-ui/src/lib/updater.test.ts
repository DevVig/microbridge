import { describe, expect, it } from "vitest";
import { automaticUpdateCheckDue } from "./updater";

const DAY = 24 * 60 * 60 * 1000;

describe("automaticUpdateCheckDue", () => {
  it("checks when there is no prior attempt", () => {
    expect(automaticUpdateCheckDue(null, DAY)).toBe(true);
  });

  it("waits for 24 hours between attempts", () => {
    expect(automaticUpdateCheckDue(String(DAY), DAY * 2 - 1)).toBe(false);
    expect(automaticUpdateCheckDue(String(DAY), DAY * 2)).toBe(true);
  });

  it("recovers when the system clock moves backwards", () => {
    expect(automaticUpdateCheckDue(String(DAY * 2), DAY)).toBe(true);
  });
});

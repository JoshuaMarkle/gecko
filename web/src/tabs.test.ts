import { describe, expect, it } from "vitest";
import { asView, isOptimizeTab } from "./tabs";

describe("asView", () => {
  it("maps renamed tabs", () => {
    expect(asView("corpus")).toBe("language");
    expect(asView("stats")).toBe("layout");
    expect(asView("analysis")).toBe("layout");
    expect(asView("search")).toBe("layout");
    expect(asView("saved")).toBe("runs");
    expect(asView("optimize")).toBe("layout");
  });
  it("keeps current tabs and defaults unknown ones to home", () => {
    expect(asView("hand")).toBe("hand");
    expect(asView("both")).toBe("both");
    expect(asView("runs")).toBe("runs");
    expect(asView("export")).toBe("export");
    expect(asView("nope")).toBe("home");
  });
  it("knows the optimize tabs", () => {
    expect(isOptimizeTab("shape")).toBe(true);
    expect(isOptimizeTab("runs")).toBe(false);
  });
});

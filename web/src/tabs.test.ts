import { describe, expect, it } from "vitest";
import { asView } from "./tabs";

describe("asView", () => {
  it("maps renamed tabs", () => {
    expect(asView("corpus")).toBe("language");
    expect(asView("stats")).toBe("analysis");
    expect(asView("runs")).toBe("saved");
    expect(asView("optimize")).toBe("home");
  });
  it("keeps current tabs and defaults unknown ones to home", () => {
    expect(asView("hand")).toBe("hand");
    expect(asView("export")).toBe("export");
    expect(asView("nope")).toBe("home");
  });
});

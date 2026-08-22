import { describe, expect, it } from "vitest";
import {
  describeProjectScope,
  encodeProjectScope,
  formatBytes,
  formatTime,
} from "../../src/api/contract";

describe("contract helpers", () => {
  it("encodes all-project and specified-project scopes as the server JSON shape", () => {
    expect(encodeProjectScope("all")).toBe("All");
    expect(encodeProjectScope([2, 7])).toEqual({ Specified: [2, 7] });
  });

  it("describes project scope for display", () => {
    expect(describeProjectScope("All")).toContain("全部项目");
    expect(describeProjectScope({ Specified: [3] })).toContain("指定项目");
    expect(describeProjectScope({ Specified: [3] })).toContain("3");
    expect(describeProjectScope({ Specified: [] })).toBe("无");
  });

  it("formats byte sizes across units and bounds", () => {
    expect(formatBytes(0)).toBe("0 B");
    expect(formatBytes(1023)).toBe("1023 B");
    expect(formatBytes(1024)).toBe("1.0 KB");
    expect(formatBytes(-1)).toBe("-1");
  });

  it("renders local time and falls back for invalid timestamps", () => {
    expect(formatTime("2026-08-20T00:00:00Z")).toMatch(/\d{4}/);
    expect(formatTime("not-a-date")).toBe("not-a-date");
  });
});

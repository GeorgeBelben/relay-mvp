import { describe, expect, it } from "vitest";
import { formatBytes } from "./formatBytes";

describe("formatBytes", () => {
  it("shows 0 B for zero or negative input", () => {
    expect(formatBytes(0)).toBe("0 B");
    expect(formatBytes(-5)).toBe("0 B");
  });

  it("shows whole bytes with no decimal", () => {
    expect(formatBytes(512)).toBe("512 B");
  });

  it("steps up through KB/MB/GB/TB with one decimal place", () => {
    expect(formatBytes(1536)).toBe("1.5 KB");
    expect(formatBytes(1024 * 1024 * 42.3)).toBe("42.3 MB");
    expect(formatBytes(1024 ** 3 * 8.75)).toBe("8.8 GB");
    expect(formatBytes(1024 ** 4 * 1.2)).toBe("1.2 TB");
  });

  it("never exceeds TB as a unit", () => {
    expect(formatBytes(1024 ** 5)).toBe("1024.0 TB");
  });
});

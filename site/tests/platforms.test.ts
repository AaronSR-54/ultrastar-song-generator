import { describe, expect, it } from "vitest";

import { detectPlatform } from "@/lib/platforms";

const WINDOWS_UA =
  "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0 Safari/537.36";
const MAC_UA =
  "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.0 Safari/605.1.15";
const LINUX_UA =
  "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0 Safari/537.36";
const IPAD_UA =
  "Mozilla/5.0 (iPad; CPU OS 17_0 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.0 Mobile/15E148 Safari/604.1";

describe("detectPlatform", () => {
  it("recommends the Windows build with Client Hints", () => {
    const result = detectPlatform({
      userAgent: WINDOWS_UA,
      uaData: { platform: "Windows", architecture: "x86", bitness: "64" },
    });
    expect(result.os).toBe("windows");
    expect(result.asset).toBe("windows-x86_64");
    expect(result.confidence).toBe("high");
  });

  it("reports Windows ARM without an asset", () => {
    const result = detectPlatform({
      userAgent: WINDOWS_UA,
      uaData: { platform: "Windows", architecture: "arm" },
    });
    expect(result.asset).toBeNull();
    expect(result.arch).toBe("arm64");
    expect(result.reason).toMatch(/Windows ARM/);
  });

  it("recommends Apple Silicon on macOS", () => {
    const result = detectPlatform({
      userAgent: MAC_UA,
      uaData: { platform: "macOS", architecture: "arm" },
    });
    expect(result.asset).toBe("macos-aarch64");
    expect(result.arch).toBe("arm64");
  });

  it("does not offer the Apple Silicon ZIP to Intel Macs", () => {
    const result = detectPlatform({
      userAgent: MAC_UA,
      uaData: { platform: "macOS", architecture: "x86", bitness: "64" },
    });
    expect(result.asset).toBeNull();
    expect(result.reason).toMatch(/Mac Intel/);
  });

  it("falls back to Safari's user agent and recommends Apple Silicon", () => {
    const result = detectPlatform({ userAgent: MAC_UA });
    expect(result.asset).toBe("macos-aarch64");
    expect(result.confidence).toBe("low");
    expect(result.reason).toMatch(/Apple Silicon/);
  });

  it("detects iPadOS through touch points on a Macintosh user agent", () => {
    const result = detectPlatform({
      userAgent: MAC_UA,
      maxTouchPoints: 5,
    });
    expect(result.os).toBe("ipados");
    expect(result.asset).toBeNull();
  });

  it("detects iPadOS from its own user agent", () => {
    const result = detectPlatform({ userAgent: IPAD_UA, maxTouchPoints: 5 });
    expect(result.os).toBe("ipados");
    expect(result.asset).toBeNull();
  });

  it("recommends the Linux build", () => {
    const result = detectPlatform({ userAgent: LINUX_UA });
    expect(result.asset).toBe("linux-x86_64");
    expect(result.confidence).toBe("low");
  });

  it("reports Linux ARM without an asset", () => {
    const result = detectPlatform({
      userAgent: LINUX_UA,
      uaData: { platform: "Linux", architecture: "arm" },
    });
    expect(result.asset).toBeNull();
    expect(result.arch).toBe("arm64");
  });

  it("rejects Android and iOS devices", () => {
    expect(
      detectPlatform({
        userAgent:
          "Mozilla/5.0 (Linux; Android 14; Pixel 8) AppleWebKit/537.36 Chrome/131.0 Mobile Safari/537.36",
        maxTouchPoints: 5,
      }).os,
    ).toBe("android");

    expect(
      detectPlatform({
        userAgent:
          "Mozilla/5.0 (iPhone; CPU iPhone OS 17_0 like Mac OS X) AppleWebKit/605.1.15 Mobile/15E148 Safari/604.1",
        maxTouchPoints: 5,
      }).os,
    ).toBe("ios");
  });

  it("does not mistake Chrome OS for Linux", () => {
    const result = detectPlatform({
      userAgent:
        "Mozilla/5.0 (X11; CrOS x86_64 14541.0.0) AppleWebKit/537.36 Chrome/131.0 Safari/537.36",
    });
    expect(result.os).toBe("unknown");
    expect(result.asset).toBeNull();
  });

  it("returns unknown for an unrecognised system", () => {
    const result = detectPlatform({ userAgent: "curl/8.0" });
    expect(result.os).toBe("unknown");
    expect(result.asset).toBeNull();
    expect(result.reason).toMatch(/Elige una plataforma/);
  });
});

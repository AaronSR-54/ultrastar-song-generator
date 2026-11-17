import { afterEach, describe, expect, it, vi } from "vitest";

import { getLatestRelease } from "@/lib/downloads";
import { ASSETS } from "@/lib/platforms";

function release(extra: string[] = [], sizes = 12_000_000) {
  return {
    ok: true,
    json: async () => ({
      tag_name: "v0.1.0",
      published_at: "2026-11-17T10:00:00Z",
      assets: [
        ...Object.values(ASSETS).map((asset) => ({
          name: asset.file,
          size: sizes,
        })),
        ...extra.map((name) => ({ name, size: 1200 })),
      ],
    }),
  };
}

afterEach(() => {
  vi.unstubAllGlobals();
});

describe("getLatestRelease", () => {
  it("returns the three stable asset URLs when the release is complete", async () => {
    vi.stubGlobal("fetch", vi.fn().mockResolvedValue(release(["SHA256SUMS.txt"])));

    const state = await getLatestRelease();

    expect(state.status).toBe("available");
    if (state.status !== "available") return;

    expect(state.release.version).toBe("v0.1.0");
    expect(state.release.assets).toHaveLength(3);
    expect(state.release.checksumsUrl).toContain("/latest/download/SHA256SUMS.txt");
    for (const asset of state.release.assets) {
      expect(asset.url).toBe(
        `https://github.com/AaronSR-54/ultrastar-song-generator/releases/latest/download/${ASSETS[asset.key].file}`,
      );
      expect(asset.size).toBe(12_000_000);
    }
  });

  it("keeps the checksum link optional", async () => {
    vi.stubGlobal("fetch", vi.fn().mockResolvedValue(release()));

    const state = await getLatestRelease();

    expect(state.status).toBe("available");
    if (state.status === "available") {
      expect(state.release.checksumsUrl).toBeNull();
    }
  });

  it("reports a release without every ZIP as missing-assets", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn().mockResolvedValue({
        ok: true,
        json: async () => ({
          tag_name: "v0.1.0",
          assets: [{ name: ASSETS["windows-x86_64"].file, size: 10 }],
        }),
      }),
    );

    const state = await getLatestRelease();

    expect(state).toEqual({ status: "missing-assets", version: "v0.1.0" });
  });

  it("returns unavailable when the API fails", async () => {
    vi.stubGlobal("fetch", vi.fn().mockResolvedValue({ ok: false }));

    expect(await getLatestRelease()).toEqual({ status: "unavailable" });
  });

  it("returns unavailable when the request throws", async () => {
    vi.stubGlobal("fetch", vi.fn().mockRejectedValue(new Error("offline")));

    expect(await getLatestRelease()).toEqual({ status: "unavailable" });
  });
});

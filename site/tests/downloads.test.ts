import { afterEach, describe, expect, it, vi } from "vitest";

import { getLatestRelease } from "@/lib/downloads";
import {
  ASSETS,
  INSTALLER_CHECKSUMS_FILE,
  type AssetKey,
} from "@/lib/platforms";

// Installer names keep the product version, unlike the stable ZIP names.
const INSTALLERS: Record<AssetKey, string> = {
  "windows-x86_64": "UltraStar-Song-Generator-0.1.0-windows-x64.msi",
  "macos-aarch64": "UltraStar-Song-Generator-0.1.0-macos-arm64.dmg",
  "linux-x86_64": "ultrastar-song-generator_0.1.0_amd64.deb",
};

function release({
  extra = [],
  sizes = 12_000_000,
  installers = true,
}: {
  extra?: string[];
  sizes?: number;
  installers?: boolean;
} = {}) {
  const native = installers
    ? Object.values(INSTALLERS).map((name) => ({ name, size: 7_500_000 }))
    : [];

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
        ...native,
        ...extra.map((name) => ({ name, size: 1200 })),
      ],
    }),
  };
}

afterEach(() => {
  vi.unstubAllGlobals();
});

describe("getLatestRelease", () => {
  it("returns the three stable ZIP URLs when the release is complete", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn().mockResolvedValue(release({ extra: ["SHA256SUMS.txt"] })),
    );

    const state = await getLatestRelease();

    expect(state.status).toBe("available");
    if (state.status !== "available") return;

    expect(state.release.version).toBe("v0.1.0");
    expect(state.release.assets).toHaveLength(3);
    expect(state.release.checksumsUrl).toContain(
      "/latest/download/SHA256SUMS.txt",
    );
    for (const asset of state.release.assets) {
      expect(asset.url).toBe(
        `https://github.com/AaronSR-54/ultrastar-song-generator/releases/latest/download/${ASSETS[asset.key].file}`,
      );
      expect(asset.size).toBe(12_000_000);
    }
  });

  it("matches the native installers even when their names carry the version", async () => {
    vi.stubGlobal("fetch", vi.fn().mockResolvedValue(release()));

    const state = await getLatestRelease();

    expect(state.status).toBe("available");
    if (state.status !== "available") return;

    expect(state.release.installers).toHaveLength(3);
    for (const installer of state.release.installers) {
      expect(installer.url).toBe(
        `https://github.com/AaronSR-54/ultrastar-song-generator/releases/latest/download/${INSTALLERS[installer.key]}`,
      );
      expect(installer.size).toBe(7_500_000);
    }
  });

  it("exposes the installer checksum file when the release publishes it", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn().mockResolvedValue(release({ extra: [INSTALLER_CHECKSUMS_FILE] })),
    );

    const state = await getLatestRelease();

    expect(state.status).toBe("available");
    if (state.status === "available") {
      expect(state.release.installerChecksumsUrl).toContain(
        `/latest/download/${INSTALLER_CHECKSUMS_FILE}`,
      );
    }
  });

  it("keeps the checksum links optional", async () => {
    vi.stubGlobal("fetch", vi.fn().mockResolvedValue(release()));

    const state = await getLatestRelease();

    expect(state.status).toBe("available");
    if (state.status === "available") {
      expect(state.release.checksumsUrl).toBeNull();
      expect(state.release.installerChecksumsUrl).toBeNull();
    }
  });

  it("still offers the ZIPs when a release has no installers", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn().mockResolvedValue(release({ installers: false })),
    );

    const state = await getLatestRelease();

    expect(state.status).toBe("available");
    if (state.status === "available") {
      expect(state.release.assets).toHaveLength(3);
      expect(state.release.installers).toEqual([]);
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

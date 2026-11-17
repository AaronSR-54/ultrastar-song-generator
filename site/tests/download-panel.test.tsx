import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";

import { DownloadPanel } from "@/components/download-panel";
import { ASSET_KEYS, ASSETS, type AssetKey } from "@/lib/platforms";
import type { ReleaseState } from "@/lib/release";

const INSTALLERS: Record<AssetKey, string> = {
  "windows-x86_64": "UltraStar-Song-Generator-0.1.0-windows-x64.msi",
  "macos-aarch64": "UltraStar-Song-Generator-0.1.0-macos-arm64.dmg",
  "linux-x86_64": "ultrastar-song-generator_0.1.0_amd64.deb",
};

const base =
  "https://github.com/AaronSR-54/ultrastar-song-generator/releases/latest/download";

const available: ReleaseState = {
  status: "available",
  release: {
    version: "v0.1.0",
    publishedAt: "2026-11-17T10:00:00Z",
    assets: ASSET_KEYS.map((key) => ({
      key,
      url: `${base}/${ASSETS[key].file}`,
      size: 42 * 1024 * 1024,
    })),
    installers: ASSET_KEYS.map((key) => ({
      key,
      url: `${base}/${INSTALLERS[key]}`,
      size: 18 * 1024 * 1024,
    })),
    checksumsUrl: `${base}/SHA256SUMS.txt`,
    installerChecksumsUrl: `${base}/SHA256SUMS-installers.txt`,
  },
};

function expectNoAssetLinks(html: string) {
  for (const key of ASSET_KEYS) {
    expect(html).not.toContain(`download/${ASSETS[key].file}`);
    expect(html).not.toContain(`download/${INSTALLERS[key]}`);
  }
}

describe("DownloadPanel server markup", () => {
  it("renders the native installer and the ZIP for every platform", () => {
    const html = renderToStaticMarkup(<DownloadPanel release={available} />);

    for (const key of ASSET_KEYS) {
      expect(html).toContain(`download/${ASSETS[key].file}`);
      expect(html).toContain(`download/${INSTALLERS[key]}`);
    }
    expect(html).toContain("Instalador MSI");
    expect(html).toContain("Instalador DMG");
    expect(html).toContain("Instalador DEB");
    expect(html).toContain("ZIP portable");
    expect(html).toContain("Elegir descarga");
    expect(html).toContain("SHA256SUMS.txt");
    expect(html).toContain("SHA256SUMS-installers.txt");
    expect(html).toContain('aria-live="polite"');
  });

  it("does not link to assets when the release has none", () => {
    const html = renderToStaticMarkup(
      <DownloadPanel release={{ status: "missing-assets", version: "v0.1.0" }} />,
    );

    expectNoAssetLinks(html);
    expect(html).toContain("Todavía no hay binarios publicados");
  });

  it("marks installers as unavailable but keeps the ZIPs in an older release", () => {
    const html = renderToStaticMarkup(
      <DownloadPanel
        release={{
          status: "available",
          release: {
            ...available.release,
            installers: [],
            installerChecksumsUrl: null,
          },
        }}
      />,
    );

    for (const key of ASSET_KEYS) {
      expect(html).toContain(`download/${ASSETS[key].file}`);
      expect(html).not.toContain(`download/${INSTALLERS[key]}`);
    }
    expect(html).toContain("no disponible");
  });

  it("falls back to the releases page when the API is unavailable", () => {
    const html = renderToStaticMarkup(
      <DownloadPanel release={{ status: "unavailable" }} />,
    );

    expectNoAssetLinks(html);
    expect(html).toContain("No hemos podido comprobar la última versión");
  });
});

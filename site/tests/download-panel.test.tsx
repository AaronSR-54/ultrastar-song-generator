import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";

import { DownloadPanel } from "@/components/download-panel";
import { ASSET_KEYS, ASSETS } from "@/lib/platforms";
import type { ReleaseState } from "@/lib/release";

const available: ReleaseState = {
  status: "available",
  release: {
    version: "v0.1.0",
    publishedAt: "2026-11-17T10:00:00Z",
    assets: ASSET_KEYS.map((key) => ({
      key,
      url: `https://github.com/AaronSR-54/ultrastar-song-generator/releases/latest/download/${ASSETS[key].file}`,
      size: 42 * 1024 * 1024,
    })),
    checksumsUrl:
      "https://github.com/AaronSR-54/ultrastar-song-generator/releases/latest/download/SHA256SUMS.txt",
  },
};

function expectNoAssetLinks(html: string) {
  for (const key of ASSET_KEYS) {
    expect(html).not.toContain(
      `releases/latest/download/${ASSETS[key].file}`,
    );
  }
}

describe("DownloadPanel server markup", () => {
  it("renders the three downloads without JavaScript", () => {
    const html = renderToStaticMarkup(<DownloadPanel release={available} />);

    for (const key of ASSET_KEYS) {
      expect(html).toContain(
        `releases/latest/download/${ASSETS[key].file}`,
      );
    }
    expect(html).toContain("Elegir descarga");
    expect(html).toContain("SHA256SUMS.txt");
    expect(html).toContain('aria-live="polite"');
  });

  it("does not link to assets when the release has none", () => {
    const html = renderToStaticMarkup(
      <DownloadPanel release={{ status: "missing-assets", version: "v0.1.0" }} />,
    );

    expectNoAssetLinks(html);
    expect(html).toContain("Todavía no hay binarios publicados");
  });

  it("falls back to the releases page when the API is unavailable", () => {
    const html = renderToStaticMarkup(
      <DownloadPanel release={{ status: "unavailable" }} />,
    );

    expectNoAssetLinks(html);
    expect(html).toContain("No hemos podido comprobar la última versión");
  });
});

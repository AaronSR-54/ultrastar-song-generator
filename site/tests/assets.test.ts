import { readFileSync } from "node:fs";
import { resolve } from "node:path";

import { describe, expect, it } from "vitest";

import { ASSET_KEYS, ASSETS, CHECKSUMS_FILE } from "@/lib/platforms";

const workflow = readFileSync(
  resolve(process.cwd(), "../.github/workflows/release.yml"),
  "utf8",
);

describe("release assets", () => {
  it("publishes every landing asset under its stable name", () => {
    for (const key of ASSET_KEYS) {
      expect(workflow).toContain(ASSETS[key].file);
    }
    expect(workflow).toContain(CHECKSUMS_FILE);
  });

  it("publishes the native installers with their own checksums", () => {
    expect(workflow).toContain("dist/installers/*.deb");
    expect(workflow).toContain("dist/installers/*.dmg");
    expect(workflow).toContain("dist/installers/*.msi");
    expect(workflow).toContain("SHA256SUMS-installers.txt");
  });

  it("creates the ZIPs from the built binaries", () => {
    expect(workflow).toMatch(/zip -j/);
    expect(workflow).toMatch(/binary-linux/);
    expect(workflow).toMatch(/binary-macos/);
    expect(workflow).toMatch(/binary-windows/);
  });

  it("publishes the four assets with the official action", () => {
    expect(workflow).toContain("softprops/action-gh-release@v2");
    expect(workflow).toContain("fail_on_unmatched_files: true");
    expect(workflow).toMatch(/refs\/tags\/v/);
  });
});

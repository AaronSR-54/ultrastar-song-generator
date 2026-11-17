import { readFileSync } from "node:fs";
import { resolve } from "node:path";

import { describe, expect, it } from "vitest";

const root = resolve(process.cwd());
const paletteSource = readFileSync(
  resolve(root, "../crates/desktop/src/theme/palette.rs"),
  "utf8",
);
const tokensSource = readFileSync(
  resolve(root, "src/styles/tokens.css"),
  "utf8",
);

function rustColors(source: string): string[] {
  return Array.from(
    source.matchAll(
      /Color32::from_rgb\(\s*0x([0-9a-fA-F]{2})\s*,\s*0x([0-9a-fA-F]{2})\s*,\s*0x([0-9a-fA-F]{2})\s*\)/g,
    ),
    (match) => `#${match[1]}${match[2]}${match[3]}`.toLowerCase(),
  );
}

function cssColors(source: string): string[] {
  return Array.from(
    source.matchAll(/#([0-9a-fA-F]{6})\b/g),
    (match) => `#${match[1]}`.toLowerCase(),
  );
}

describe("design tokens", () => {
  const palette = new Set(rustColors(paletteSource));
  const css = new Set(cssColors(tokensSource));

  it("parses the app palette", () => {
    expect(palette.size).toBeGreaterThanOrEqual(20);
  });

  it("includes every app colour in the landing tokens", () => {
    const missing = [...palette].filter((color) => !css.has(color));
    expect(missing).toEqual([]);
  });

  it("does not introduce colours outside the app palette", () => {
    const extra = [...css].filter((color) => !palette.has(color));
    expect(extra).toEqual([]);
  });

  it("keeps square corners", () => {
    expect(paletteSource).toMatch(/pub const RADIUS: u8 = 0;/);
    expect(tokensSource).toMatch(/--radius:\s*0px;/);
  });

  it("keeps the type scale anchored to the app", () => {
    expect(tokensSource).toMatch(/--text-micro:\s*0\.6875rem;/);
    expect(tokensSource).toMatch(/--text-caption:\s*0\.78125rem;/);
    expect(tokensSource).toMatch(/--text-body:\s*0\.875rem;/);
    expect(tokensSource).toMatch(/--text-title:\s*1rem;/);
    expect(tokensSource).toMatch(/--text-display:\s*1\.25rem;/);
    expect(tokensSource).toMatch(/--text-hero:\s*1\.5rem;/);
  });
});

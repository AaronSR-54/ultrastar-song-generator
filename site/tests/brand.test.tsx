import { readFileSync } from "node:fs";
import { resolve } from "node:path";

import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";

import { BrandLogo } from "@/components/brand-mark";

const icon = readFileSync(resolve(process.cwd(), "app/icon.svg"), "utf8");

describe("brand mark", () => {
  it("paints the microphone without a tile or background", () => {
    const html = renderToStaticMarkup(<BrandLogo />);

    expect(html).toContain("<svg");
    expect(html).toContain("text-accent-text");
    expect(html).not.toContain("bg-accent/15");
    expect(html).not.toContain("border-accent/30");
  });

  it("keeps the favicon transparent", () => {
    expect(icon).not.toContain("<rect");
    expect(icon).toContain('viewBox="2 2 20 20"');
    expect(icon).toContain('stroke="#4F8DFF"');
  });
});

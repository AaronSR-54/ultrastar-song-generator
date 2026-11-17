import { readFileSync } from "node:fs";
import { resolve } from "node:path";

import { describe, expect, it } from "vitest";

const globalsSource = readFileSync(
  resolve(process.cwd(), "app/globals.css"),
  "utf8",
);

function ruleFor(selector: string): string {
  const index = globalsSource.indexOf(selector);
  expect(index, `falta la regla ${selector}`).toBeGreaterThan(-1);

  const start = globalsSource.indexOf("{", index);
  const end = globalsSource.indexOf("}", start);
  return globalsSource.slice(start + 1, end);
}

describe("site header styles", () => {
  it("defines the base glass layer with the app shadow", () => {
    const base = ruleFor(".site-header::before");

    expect(base).toContain("background-color: rgb(10 10 12 / 0.94)");
    expect(base).toContain("box-shadow: var(--app-shadow)");
    expect(base).toContain("backdrop-filter: blur(12px) saturate(140%)");
    expect(base).toContain("transition: opacity 180ms ease");
  });

  it("turns the glass off at the top of the page", () => {
    const top = ruleFor(
      'html[data-js="true"] .site-header[data-scrolled="false"]::before',
    );

    expect(top).toContain("background-color: transparent");
    expect(top).toContain("box-shadow: none");
    expect(top).toContain("backdrop-filter: none");
    expect(top).toContain("opacity: 0");
  });

  it("uses a lighter tint while scrolled", () => {
    const scrolled = ruleFor(
      'html[data-js="true"] .site-header[data-scrolled="true"]::before',
    );

    expect(scrolled).toContain("background-color: rgb(10 10 12 / 0.72)");
  });

  it("opacifies the glass when backdrop-filter is unsupported", () => {
    const fallback = ruleFor("@supports not (");

    expect(fallback).toContain("background-color: rgb(10 10 12 / 0.94)");
    expect(fallback).toContain("backdrop-filter: none");
  });

  it("keeps the header free of bottom borders in every state", () => {
    const selectors = [
      ".site-header::before",
      'html[data-js="true"] .site-header[data-scrolled="false"]::before',
      'html[data-js="true"] .site-header[data-scrolled="true"]::before',
    ];

    for (const selector of selectors) {
      expect(ruleFor(selector)).not.toMatch(/border-(bottom|color|width)/);
    }
  });
});

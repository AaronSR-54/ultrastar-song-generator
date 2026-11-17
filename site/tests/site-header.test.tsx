import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";

import { SiteHeader } from "@/components/site-header";

describe("SiteHeader server markup", () => {
  const html = renderToStaticMarkup(<SiteHeader />);

  it("renders no bottom border or opaque background", () => {
    expect(html).not.toContain("border-b");
    expect(html).not.toContain("border-border");
    expect(html).not.toContain("bg-bg/95");
  });

  it("exposes the scroll state hook for the client island", () => {
    expect(html).toContain("data-site-header");
    expect(html).toContain('data-scrolled="false"');
    expect(html).toContain("site-header");
  });

  it("keeps the navigation anchors and the download CTA", () => {
    expect(html).toContain('href="#como-funciona"');
    expect(html).toContain('href="#requisitos"');
    expect(html).toContain('href="#descargas"');
    expect(html).toContain("Cómo funciona");
    expect(html).toContain("Requisitos");
    expect(html).toContain("Descargar");
  });

  it("keeps the header height unchanged", () => {
    expect(html).toContain("h-16");
  });
});

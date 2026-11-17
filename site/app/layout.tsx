import type { Metadata, Viewport } from "next";

import { RevealProvider } from "@/components/reveal";
import { SiteFooter } from "@/components/site-footer";
import { SiteHeader } from "@/components/site-header";
import { siteUrl, SITE_DESCRIPTION } from "@/lib/site";

import "./globals.css";

const description = SITE_DESCRIPTION;

export const metadata: Metadata = {
  metadataBase: new URL(siteUrl()),
  title: "UltraStar Song Generator — de YouTube a canción jugable",
  description,
  alternates: { canonical: "/" },
  robots: { index: true, follow: true },
  openGraph: {
    type: "website",
    locale: "es_ES",
    url: "/",
    siteName: "UltraStar Song Generator",
    title: "UltraStar Song Generator — de YouTube a canción jugable",
    description,
    images: [
      {
        url: "/og.png",
        width: 1200,
        height: 630,
        alt: "UltraStar Song Generator: de un enlace de YouTube a una canción jugable.",
      },
    ],
  },
  twitter: {
    card: "summary_large_image",
    title: "UltraStar Song Generator — de YouTube a canción jugable",
    description,
    images: ["/og.png"],
  },
};

export const viewport: Viewport = {
  themeColor: "#0a0a0c",
  colorScheme: "dark",
};

const setJsFlag = 'document.documentElement.dataset.js = "true";';

export default function RootLayout({
  children,
}: Readonly<{ children: React.ReactNode }>) {
  return (
    <html lang="es">
      <head>
        <script dangerouslySetInnerHTML={{ __html: setJsFlag }} />
      </head>
      <body>
        <a
          href="#main"
          className="sr-only focus:not-sr-only focus:fixed focus:top-3 focus:left-3 focus:z-[100] focus:border focus:border-accent focus:bg-bg focus:px-4 focus:py-2 focus:text-body focus:text-text"
        >
          Saltar al contenido
        </a>
        <SiteHeader />
        <main id="main">{children}</main>
        <SiteFooter />
        <RevealProvider />
      </body>
    </html>
  );
}

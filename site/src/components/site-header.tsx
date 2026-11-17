import { BrandLogo } from "@/components/brand-mark";
import { ButtonLink } from "@/components/ui/button";
import { HeaderScrollState } from "@/components/header-scroll-state";
import { Icon } from "@/components/ui/icon";

const nav = [
  { href: "#como-funciona", label: "Cómo funciona" },
  { href: "#requisitos", label: "Requisitos" },
  { href: "#descargas", label: "Descargas" },
];

export function SiteHeader() {
  return (
    <header
      className="site-header sticky top-0 z-50"
      data-site-header
      data-scrolled="false"
    >
      <HeaderScrollState />
      <div className="relative z-10 mx-auto flex h-16 w-full max-w-[1200px] items-center justify-between gap-4 px-6 md:px-10 lg:px-12">
        <a
          href="#top"
          className="flex min-w-0 items-center gap-3"
          aria-label="UltraStar Song Generator, volver arriba"
        >
          <BrandLogo size={30} />
          <span className="flex min-w-0 flex-col leading-none">
            <span className="truncate text-[13px] font-semibold tracking-[-0.01em]">
              <span className="sm:hidden">UltraStar</span>
              <span className="hidden sm:inline">UltraStar Song Generator</span>
            </span>
            <span className="mt-1 hidden font-mono text-[9px] uppercase tracking-[0.16em] text-text-3 sm:block">
              USDX // local pipeline
            </span>
          </span>
        </a>

        <nav aria-label="Secciones" className="hidden items-center gap-7 md:flex">
          {nav.map((item) => (
            <a
              key={item.href}
              href={item.href}
              className="text-body text-text-2 transition-colors duration-150 hover:text-text"
            >
              {item.label}
            </a>
          ))}
        </nav>

        <div className="flex shrink-0 items-center gap-1">
          <a
            href="https://github.com/AaronSR-54/ultrastar-song-generator"
            target="_blank"
            rel="noreferrer noopener"
            aria-label="Abrir el repositorio en GitHub"
            className="grid size-9 place-items-center text-text-2 transition-colors duration-150 hover:bg-surface-2 hover:text-text"
          >
            <Icon name="external" size={16} />
          </a>
          <ButtonLink href="#descargas" size="sm" icon="download">
            Descargar
          </ButtonLink>
        </div>
      </div>
    </header>
  );
}

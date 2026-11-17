import { Icon } from "@/components/ui/icon";
import { MonoLabel } from "@/components/ui/mono-label";

const links = [
  {
    href: "https://github.com/AaronSR-54/ultrastar-song-generator",
    label: "Repositorio",
  },
  {
    href: "https://github.com/AaronSR-54/ultrastar-song-generator/tree/main/docs",
    label: "Documentación",
  },
  {
    href: "https://github.com/AaronSR-54/ultrastar-song-generator/releases",
    label: "Versiones",
  },
  {
    href: "https://github.com/AaronSR-54/ultrastar-song-generator#licencia",
    label: "Licencia",
  },
];

export function SiteFooter() {
  return (
    <footer className="border-t border-border">
      <div className="mx-auto w-full max-w-[1200px] px-6 py-12 md:px-10 lg:px-12">
        <div className="flex flex-col gap-10 md:flex-row md:items-start md:justify-between">
          <div className="flex items-center gap-3">
            <span className="grid size-8 place-items-center border border-accent/30 bg-accent/15">
              <Icon name="music" size={16} className="text-accent-text" />
            </span>
            <div>
              <p className="text-body font-semibold">UltraStar Song Generator</p>
              <p className="mt-1 text-caption text-text-3">
                De un enlace de YouTube a una canción jugable.
              </p>
            </div>
          </div>

          <nav aria-label="Enlaces del proyecto">
            <ul className="flex flex-wrap gap-x-6 gap-y-3">
              {links.map((link) => (
                <li key={link.href}>
                  <a
                    href={link.href}
                    target="_blank"
                    rel="noreferrer noopener"
                    className="inline-flex items-center gap-1.5 text-body text-text-2 transition-colors duration-150 hover:text-text"
                  >
                    {link.label}
                    <Icon name="external" size={12} className="text-text-3" />
                  </a>
                </li>
              ))}
            </ul>
          </nav>
        </div>

        <div className="mt-10 flex flex-col gap-3 border-t border-border pt-6 text-caption text-text-3 md:flex-row md:items-center md:justify-between">
          <p>MIT OR Apache-2.0 · Sin telemetría · Sin cuentas</p>
          <MonoLabel className="text-[10px] tracking-[0.16em]">
            Hecho en local. Como la app.
          </MonoLabel>
        </div>
      </div>
    </footer>
  );
}

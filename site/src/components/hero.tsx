import { AppWindow } from "@/components/app-window";
import { ButtonLink } from "@/components/ui/button";
import { Icon } from "@/components/ui/icon";
import { MonoLabel } from "@/components/ui/mono-label";
import { cn } from "@/lib/cn";

const promises = [
  "Sin cuenta",
  "Sin telemetría",
  "El audio no sale de tu equipo",
];

const specs = [
  { value: "06", label: "etapas" },
  { value: "48", label: "kHz de análisis" },
  { value: "01", label: "voz, sin duetos" },
  { value: "00", label: "subidas de audio" },
];

export function Hero() {
  return (
    <section id="top" className="relative overflow-hidden">
      <div className="dot-field" aria-hidden="true" />
      <div
        aria-hidden="true"
        className="pointer-events-none absolute inset-x-0 top-0 h-[560px]"
        style={{
          background:
            "radial-gradient(ellipse 70% 100% at 50% 0%, rgb(21 22 27 / 0.9) 0%, transparent 70%)",
        }}
      />

      <div className="relative mx-auto w-full max-w-[1200px] px-6 pt-14 pb-0 md:px-10 md:pt-20 lg:px-12 lg:pt-24">
        <div className="grid gap-12 lg:grid-cols-12 lg:gap-10">
          <div className="lg:col-span-5 xl:col-span-6">
            <div className="flex items-center gap-4">
              <span className="border border-border px-2 py-1 font-mono text-[10px] tracking-[0.14em] text-text-2">
                v0.1.0
              </span>
              <MonoLabel className="text-[10px] tracking-[0.16em]">
                Windows · macOS · Linux
              </MonoLabel>
              <span className="h-px flex-1 bg-border" aria-hidden="true" />
            </div>

            <h1 className="mt-7 max-w-[15ch] text-[clamp(2.5rem,6vw,4.15rem)] leading-[0.98] font-semibold tracking-[-0.038em] lg:text-[clamp(2.4rem,4.5vw,4.15rem)]">
              De un enlace de YouTube a una canción jugable.
            </h1>

            <p className="mt-7 max-w-[52ch] text-[16px] leading-[1.65] text-text-2 md:text-[17px]">
              Letra, separación de voces y pitch en tu equipo. Sale un{" "}
              <span className="font-mono text-[0.92em] text-text">.txt</span>{" "}
              de UltraStar con audio, carátula y vídeo, listo para el juego.
            </p>

            <div className="mt-8 flex flex-col gap-3 sm:flex-row">
              <ButtonLink
                href="#descargas"
                size="lg"
                icon="download"
                className="w-full sm:w-auto"
              >
                Descargar la app
              </ButtonLink>
              <ButtonLink
                href="https://github.com/AaronSR-54/ultrastar-song-generator"
                target="_blank"
                rel="noreferrer noopener"
                size="lg"
                variant="secondary"
                icon="external"
                className="w-full sm:w-auto"
              >
                Ver el código
              </ButtonLink>
            </div>

            <ul className="mt-7 flex flex-wrap items-center gap-x-5 gap-y-2">
              {promises.map((item) => (
                <li
                  key={item}
                  className="flex items-center gap-1.5 text-caption text-text-3"
                >
                  <Icon name="check" size={12} className="text-text-2" />
                  {item}
                </li>
              ))}
            </ul>
          </div>

          <div className="lg:col-span-7 lg:pt-14 xl:col-span-6">
            <AppWindow />
          </div>
        </div>

        <dl className="mt-14 grid grid-cols-2 border-t border-border md:mt-20 md:grid-cols-4">
          {specs.map((spec, index) => (
            <div
              key={spec.label}
              className={cn(
                "py-5 pr-4 md:pr-6",
                index % 2 === 1 && "border-l border-border pl-4",
                index >= 2 && "border-t border-border md:border-t-0",
                index > 0 && "md:border-l md:border-border md:pl-6",
              )}
            >
              <dt className="font-mono text-[9px] tracking-[0.16em] text-text-3 uppercase">
                {spec.label}
              </dt>
              <dd className="mt-1 text-[22px] leading-none font-semibold tracking-[-0.03em]">
                {spec.value}
              </dd>
            </div>
          ))}
        </dl>
      </div>
    </section>
  );
}

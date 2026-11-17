import { Icon } from "@/components/ui/icon";
import { MonoLabel } from "@/components/ui/mono-label";

const points = [
  {
    icon: "cpu" as const,
    title: "Demucs, en tu CPU",
    body: "HTDemucs separa la voz del instrumental sin enviar la pista a ningún servidor.",
  },
  {
    icon: "lock" as const,
    title: "Whisper, solo si hace falta",
    body: "Se descarga cuando no hay letra online y eliges transcribir. El resto se queda en disco.",
  },
  {
    icon: "youtube" as const,
    title: "A la red, solo metadatos",
    body: "Título, artista y duración viajan a LRCLIB para buscar la letra. Nada más sale del equipo.",
  },
];

function Connector({ label }: { label: string }) {
  return (
    <div className="flex items-center gap-3 py-1.5 pl-5">
      <span className="h-5 w-px bg-border" aria-hidden="true" />
      <span className="font-mono text-[9px] tracking-[0.08em] text-text-3">
        {label}
      </span>
    </div>
  );
}

export function Privacy() {
  return (
    <div className="grid gap-12 lg:grid-cols-2 lg:gap-16">
      <ol className="border-t border-border">
        {points.map((point, index) => (
          <li
            key={point.title}
            className="grid grid-cols-[32px_minmax(0,1fr)] gap-4 border-b border-border py-5"
          >
            <span className="grid size-8 place-items-center border border-border bg-surface-2 text-text-2">
              <Icon name={point.icon} size={14} />
            </span>
            <div>
              <div className="flex items-baseline gap-3">
                <MonoLabel className="text-[9px] tracking-[0.16em]">
                  {String(index + 1).padStart(2, "0")}
                </MonoLabel>
                <h3 className="text-title font-semibold tracking-[-0.01em]">
                  {point.title}
                </h3>
              </div>
              <p className="mt-2 max-w-[46ch] text-body leading-[1.65] text-text-2">
                {point.body}
              </p>
            </div>
          </li>
        ))}
      </ol>

      <div className="border border-border bg-surface/50 p-5 md:p-7">
        <div className="flex items-center justify-between gap-4">
          <MonoLabel className="text-[10px] tracking-[0.16em]">
            Flujo de datos
          </MonoLabel>
          <span className="flex items-center gap-1.5 font-mono text-[9px] tracking-[0.08em] text-text-3">
            <span className="size-1.5 bg-success" />
            audio en local
          </span>
        </div>

        <div className="mt-7">
          <div className="flex items-center gap-3 border border-border bg-bg px-4 py-3">
            <Icon name="youtube" size={16} className="text-text-3" />
            <div className="min-w-0">
              <p className="text-body font-semibold">YouTube</p>
              <p className="font-mono text-[9px] text-text-3">
                audio · vídeo · carátula
              </p>
            </div>
          </div>

          <Connector label="descarga" />

          <div className="grid gap-2 md:grid-cols-[minmax(0,1fr)_56px_132px] md:items-center">
            <div className="border border-border bg-surface-2 p-4">
              <div className="flex items-center justify-between gap-3">
                <p className="text-body font-semibold">Tu equipo</p>
                <MonoLabel className="text-[8px] tracking-[0.16em]">
                  local
                </MonoLabel>
              </div>
              <div className="mt-3 flex flex-wrap gap-1.5">
                {["Demucs", "Whisper", "pYIN", "espeak"].map((chip) => (
                  <span
                    key={chip}
                    className="border border-border px-1.5 py-0.5 font-mono text-[9px] text-text-2"
                  >
                    {chip}
                  </span>
                ))}
              </div>
            </div>

            <div className="hidden flex-col items-center gap-1 md:flex">
              <span className="font-mono text-[8px] tracking-[0.06em] text-text-3">
                metadatos
              </span>
              <span className="flex w-full items-center">
                <span className="h-px flex-1 border-t border-dashed border-border-hover" />
                <Icon name="arrow-right" size={10} className="text-text-3" />
              </span>
            </div>

            <div className="border border-dashed border-border-hover p-3">
              <p className="text-caption font-semibold">LRCLIB</p>
              <p className="mt-1 font-mono text-[8px] leading-[1.6] text-text-3">
                título · artista · duración
                <br />
                <span className="md:hidden">solo metadatos</span>
                <span className="hidden md:inline">consulta de letra</span>
              </p>
            </div>
          </div>

          <Connector label="copia local" />

          <div className="flex items-center gap-3 border border-border bg-bg px-4 py-3">
            <Icon name="folder" size={16} className="text-text-3" />
            <div className="min-w-0">
              <p className="truncate text-body font-semibold">
                songs/Artista - Título
              </p>
              <p className="font-mono text-[9px] text-text-3">
                .txt · audio.mp3 · cover.jpg · video.mp4
              </p>
            </div>
          </div>
        </div>

        <p className="mt-6 border-t border-border pt-4 text-caption leading-[1.6] text-text-3">
          El vídeo y el audio se descargan de YouTube; el procesamiento no
          vuelve a salir a la red.
        </p>
      </div>
    </div>
  );
}

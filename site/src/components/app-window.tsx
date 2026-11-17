import { Icon } from "@/components/ui/icon";
import { MonoLabel } from "@/components/ui/mono-label";

function WindowButton({ kind }: { kind: "min" | "max" | "close" }) {
  return (
    <span
      aria-hidden="true"
      className="grid h-7 w-8 place-items-center border-l border-border text-text-3 first:border-l-0"
    >
      {kind === "min" ? <span className="h-px w-2.5 bg-current" /> : null}
      {kind === "max" ? (
        <span className="size-2.5 border border-current" />
      ) : null}
      {kind === "close" ? (
        <span className="relative size-2.5">
          <span className="absolute top-1/2 left-0 h-px w-full rotate-45 bg-current" />
          <span className="absolute top-1/2 left-0 h-px w-full -rotate-45 bg-current" />
        </span>
      ) : null}
    </span>
  );
}

export function AppWindow() {
  return (
    <div className="relative">
      <div className="mb-3 flex items-end justify-between gap-4">
        <MonoLabel className="text-[10px] tracking-[0.16em]">
          App / Inicio
        </MonoLabel>
        <MonoLabel className="text-[10px] tracking-[0.16em]">
          USDX 0.1.0
        </MonoLabel>
      </div>

      <div className="border border-border bg-surface shadow-modal">
        <div className="flex h-10 items-stretch justify-between border-b border-border bg-surface-2 pl-3">
          <div className="flex min-w-0 items-center gap-2">
            <span className="grid size-[18px] shrink-0 place-items-center border border-accent/30 bg-accent/15">
              <Icon name="music" size={10} className="text-accent-text" />
            </span>
            <span className="truncate text-[11px] font-semibold tracking-[-0.01em]">
              UltraStar Song Generator
            </span>
            <span className="hidden font-mono text-[9px] tracking-[0.14em] text-text-3 sm:inline">
              USDX
            </span>
          </div>
          <div className="flex shrink-0 items-stretch">
            <WindowButton kind="min" />
            <WindowButton kind="max" />
            <WindowButton kind="close" />
          </div>
        </div>

        <div className="grid grid-cols-[40px_minmax(0,1fr)]">
          <div className="flex flex-col items-center gap-1 border-r border-border bg-surface-2/40 py-2">
            <span className="grid size-7 place-items-center border border-border bg-surface-3 text-text">
              <Icon name="music" size={13} />
            </span>
            <span className="grid size-7 place-items-center text-text-3">
              <Icon name="folder" size={13} />
            </span>
            <span className="grid size-7 place-items-center text-text-3">
              <Icon name="settings" size={13} />
            </span>
          </div>

          <div className="p-4 sm:p-6">
            <div className="flex items-center justify-between gap-3">
              <MonoLabel className="text-[9px] tracking-[0.16em]">
                Inicio
              </MonoLabel>
              <span className="flex items-center gap-1.5 font-mono text-[9px] tracking-[0.08em] text-text-3">
                <span className="size-1.5 bg-success" />
                modelos listos
              </span>
            </div>

            <p className="mt-3 text-[19px] leading-none font-semibold tracking-[-0.02em] sm:text-[21px]">
              Nueva canción
            </p>
            <p className="mt-2 max-w-[34ch] text-[11px] leading-[1.6] text-text-2">
              Pega el enlace. La app descarga, separa, mide y exporta.
            </p>

            <div className="mt-5">
              <MonoLabel className="text-[9px] tracking-[0.14em]">
                URL del vídeo
              </MonoLabel>
              <div className="mt-2 flex h-9 items-center justify-between gap-2 border border-border bg-field px-2.5">
                <span className="truncate font-mono text-[10px] text-text-3">
                  https://www.youtube.com/watch?v=…
                </span>
                <span className="shrink-0 border border-border px-1.5 font-mono text-[9px] text-text-3">
                  ↵
                </span>
              </div>
              <div className="mt-2 flex h-9 items-center justify-center gap-2 bg-accent text-[11px] font-semibold text-on-accent">
                Generar
                <Icon name="arrow-right" size={13} />
              </div>
            </div>

            <div className="mt-5 grid grid-cols-3 gap-2 border-t border-border pt-3 font-mono text-[9px] tracking-[0.04em] text-text-3">
              <span className="truncate">yt-dlp 2026.10.21</span>
              <span className="truncate">ffmpeg 8.0</span>
              <span className="truncate text-right">WAV 48 kHz</span>
            </div>
          </div>
        </div>

        <div className="flex h-8 items-center justify-between border-t border-border px-3 font-mono text-[9px] tracking-[0.08em] text-text-3">
          <span>local · sin cuentas</span>
          <span>0 subidas</span>
        </div>
      </div>

      <p className="mt-3 flex items-center justify-between gap-4 font-mono text-[10px] tracking-[0.06em] text-text-3">
        <span>Una URL. Seis etapas.</span>
        <span className="text-right">Salida: .txt USDX</span>
      </p>
    </div>
  );
}

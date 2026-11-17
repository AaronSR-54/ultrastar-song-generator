import { Icon } from "@/components/ui/icon";
import { MonoLabel } from "@/components/ui/mono-label";

const tree = [
  { name: "The Beatles - Yesterday.txt", kind: "letra + notas", size: "3,1 KB" },
  { name: "audio.mp3", kind: "pista", size: "3,8 MB" },
  { name: "cover.jpg", kind: "carátula", size: "47 KB" },
  { name: "video.mp4", kind: "vídeo", size: "1,0 MB" },
];

const header = [
  ["#TITLE", "Yesterday"],
  ["#ARTIST", "The Beatles"],
  ["#LANGUAGE", "en"],
  ["#YEAR", "2009"],
  ["#MP3", "audio.mp3"],
  ["#COVER", "cover.jpg"],
  ["#VIDEO", "video.mp4"],
  ["#BPM", "382,8125"],
  ["#GAP", "116"],
];

const notes = [
  [135, 7, -7, "Yes"],
  [142, 2, -7, "ter"],
  [144, 31, -7, "day,"],
  [213, 8, -4, "all"],
  [221, 7, -1, "my"],
  [228, 9, 1, "trou"],
  [237, 8, 2, "bles"],
  [245, 8, 4, "seemed"],
];

export function ExportPreview() {
  return (
    <div>
      <div className="grid items-start gap-10 lg:grid-cols-[minmax(0,1fr)_minmax(0,1fr)] lg:gap-12">
        <div className="min-w-0 border border-border bg-surface/50">
          <div className="flex h-10 items-center justify-between border-b border-border px-4">
            <MonoLabel className="text-[9px] tracking-[0.14em]">
              output/
            </MonoLabel>
            <span className="flex items-center gap-1.5 font-mono text-[9px] text-text-3">
              <span className="size-1.5 bg-success" />
              carpeta lista
            </span>
          </div>

          <div className="p-4 font-mono text-[11px] leading-[2] md:p-5">
            <p className="text-text">
              <Icon
                name="folder"
                size={12}
                className="mr-2 inline align-[-1px] text-text-3"
              />
              The Beatles - Yesterday/
            </p>
            {tree.map((item, index) => (
              <div
                key={item.name}
                className="flex items-baseline justify-between gap-4 pl-4"
              >
                <span className="min-w-0 truncate text-text-2">
                  <span className="text-text-3">
                    {index === tree.length - 1 ? "└── " : "├── "}
                  </span>
                  {item.name}
                </span>
                <span className="shrink-0 text-[9px] tracking-[0.06em] text-text-3 max-sm:hidden">
                  {item.size}
                </span>
              </div>
            ))}
          </div>

          <div className="border-t border-border p-4 md:p-5">
            <div className="flex items-center justify-between gap-4">
              <MonoLabel className="text-[9px] tracking-[0.14em]">
                Destino
              </MonoLabel>
              <span className="font-mono text-[9px] text-text-3">
                si está instalado
              </span>
            </div>
            <div className="mt-3 flex items-start gap-2.5">
              <Icon name="check" size={13} className="mt-1 text-success" />
              <div className="min-w-0">
                <p className="text-body font-semibold">
                  La carpeta del juego, si existe
                </p>
                <p className="mt-1 truncate font-mono text-[10px] text-text-3">
                  ~/.ultrastardx/config.ini → songs/
                </p>
              </div>
            </div>
            <p className="mt-3 max-w-[44ch] text-caption leading-[1.6] text-text-2">
              La app lee la carpeta que ya usa UltraStar Deluxe. Si el juego no
              está o usas otra ruta, elige la carpeta en Ajustes: la elección
              manual manda.
            </p>
          </div>
        </div>

        <div className="min-w-0">
          <div className="border border-border bg-field">
            <div className="flex h-10 items-center justify-between gap-4 border-b border-border bg-surface-2 px-4">
              <MonoLabel className="min-w-0 truncate text-[9px] tracking-[0.12em]">
                The Beatles - Yesterday.txt
              </MonoLabel>
              <span className="shrink-0 font-mono text-[9px] whitespace-nowrap text-text-3">
                UTF-8
              </span>
            </div>
            <div className="overflow-x-auto p-4 font-mono text-[11px] leading-[1.9] md:p-5">
              <pre className="text-text-2">
                {header.map(([key, value]) => (
                  <div key={key}>
                    <span className="text-text-3">{key}:</span>
                    <span className="text-text">{value}</span>
                  </div>
                ))}
                <div className="h-4" />
                {notes.map(([start, duration, pitch, text]) => (
                  <div key={`${start}-${text}`}>
                    <span className="text-text-3">
                      : {start} {duration} {pitch}{" "}
                    </span>
                    <span className="text-text">{text}</span>
                  </div>
                ))}
                <div className="h-4" />
                <div className="text-text-3">E</div>
              </pre>
            </div>
          </div>

          <ul className="mt-6 space-y-3">
            <li className="flex gap-2.5 text-caption leading-[1.6] text-text-2">
              <Icon name="check" size={12} className="mt-1 shrink-0 text-text-3" />
              Valida antes de escribir: una nota con texto vacío detiene la
              exportación y selecciona la culpable.
            </li>
            <li className="flex gap-2.5 text-caption leading-[1.6] text-text-2">
              <Icon name="check" size={12} className="mt-1 shrink-0 text-text-3" />
              Si solo hay avisos, exporta y los muestra; no bloquea la canción.
            </li>
          </ul>
        </div>
      </div>

      <p className="mt-8 flex flex-col gap-2 border-t border-border pt-5 font-mono text-[10px] leading-[1.8] tracking-[0.02em] text-text-3 sm:flex-row sm:items-baseline sm:justify-between">
        <span>
          #TITLE · #ARTIST · #LANGUAGE · #YEAR · #MP3 · #COVER · #VIDEO · #BPM ·
          #GAP · frases con notas · E
        </span>
        <span className="text-right">Salida real de la app</span>
      </p>
    </div>
  );
}

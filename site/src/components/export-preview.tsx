import { Icon } from "@/components/ui/icon";
import { MonoLabel } from "@/components/ui/mono-label";

const tree = [
  { name: "La ciudad enciende.txt", kind: "letra + notas", file: true },
  { name: "audio.mp3", kind: "pista", file: true },
  { name: "cover.jpg", kind: "carátula", file: true },
  { name: "video.mp4", kind: "vídeo", file: true },
];

const header = [
  ["#TITLE", "La ciudad enciende"],
  ["#ARTIST", "Demo local"],
  ["#LANGUAGE", "Spanish"],
  ["#MP3", "audio.mp3"],
  ["#COVER", "cover.jpg"],
  ["#VIDEO", "video.mp4"],
  ["#BPM", "238,4"],
  ["#GAP", "1200"],
];

const notes = [
  [0, 4, 8, "La"],
  [4, 4, 10, "ciu"],
  [8, 6, 12, "dad"],
  [14, 4, 10, "se"],
  [18, 5, 8, "en"],
  [23, 7, 11, "cien"],
  [30, 4, 9, "de"],
];

export function ExportPreview() {
  return (
    <div>
      <div className="grid gap-10 lg:grid-cols-[minmax(0,1fr)_minmax(0,1fr)] lg:gap-12">
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
              Demo local - La ciudad enciende/
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
                <span className="shrink-0 text-[9px] tracking-[0.06em] text-text-3 uppercase max-sm:hidden">
                  {item.kind}
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
                automático
              </span>
            </div>
            <div className="mt-3 flex items-start gap-2.5">
              <Icon name="check" size={13} className="mt-1 text-success" />
              <div className="min-w-0">
                <p className="text-body font-semibold">SongDir1 detectado</p>
                <p className="mt-1 truncate font-mono text-[10px] text-text-3">
                  ~/.ultrastardx/config.ini → songs/
                </p>
              </div>
            </div>
            <p className="mt-3 max-w-[44ch] text-caption leading-[1.6] text-text-2">
              Si el juego está en otra ruta, elige la carpeta en Ajustes y cada
              canción se copia sola. La elección manual manda.
            </p>
          </div>
        </div>

        <div className="min-w-0">
          <div className="border border-border bg-field">
            <div className="flex h-10 items-center justify-between gap-4 border-b border-border bg-surface-2 px-4">
              <MonoLabel className="min-w-0 truncate text-[9px] tracking-[0.12em]">
                Demo local - La ciudad enciende.txt
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

      <p className="mt-8 border-t border-border pt-5 font-mono text-[10px] leading-[1.8] tracking-[0.02em] text-text-3">
        #TITLE · #ARTIST · #LANGUAGE · #MP3 · #COVER · #VIDEO · #BPM · #GAP ·
        frases con notas · E
      </p>
    </div>
  );
}

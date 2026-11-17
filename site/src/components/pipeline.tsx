import { Icon, type IconName } from "@/components/ui/icon";
import { MonoLabel } from "@/components/ui/mono-label";

type Stage = {
  number: string;
  name: string;
  icon: IconName;
  description: string;
  output: string;
  wave?: boolean;
};

const stages: Stage[] = [
  {
    number: "01",
    name: "Ingest",
    icon: "youtube",
    description: "Metadatos, audio, carátula y vídeo. WAV mono de 48 kHz.",
    output: "source/ · work/analysis.wav",
  },
  {
    number: "02",
    name: "Separate",
    icon: "waveform",
    description: "Separa la voz del instrumental con HTDemucs, en local.",
    output: "work/stems/",
  },
  {
    number: "03",
    name: "Transcribe",
    icon: "file",
    description:
      "Busca la letra en LRCLIB. Si no hay, Whisper o la pegas tú.",
    output: "work/transcript.json",
  },
  {
    number: "04",
    name: "Pitch",
    icon: "music",
    description: "F0 y ataques, tempo y cuantización de las notas.",
    output: "work/notes.json",
  },
  {
    number: "05",
    name: "Align",
    icon: "cpu",
    description: "Fonemas y sílabas: mide dónde se canta cada una.",
    output: "work/song.json",
    wave: true,
  },
  {
    number: "06",
    name: "Export",
    icon: "disc",
    description: "Valida y escribe el .txt de UltraStar y los medios.",
    output: "output/Artista - Título/",
  },
];

export function Pipeline() {
  return (
    <div>
      <ol className="border-t border-border sm:grid sm:grid-cols-2 lg:grid-cols-6">
        {stages.map((stage) => (
          <li
            key={stage.number}
            className="relative border-b border-border py-6 sm:px-5 sm:py-8 sm:odd:border-r lg:border-r lg:border-b-0 lg:last:border-r-0"
          >
            <span
              aria-hidden="true"
className="absolute top-0 left-0 size-[9px] -translate-y-1/2 border border-border bg-bg sm:left-5"
            >
              <span
                className={
                  stage.wave
                    ? "absolute inset-[1px] bg-wave"
                    : "absolute inset-[2px] bg-border-hover"
                }
              />
            </span>

            <div className="flex items-center justify-between gap-3 pl-4 sm:pl-0">
              <MonoLabel className="text-[10px] tracking-[0.18em]">
                {stage.number}
              </MonoLabel>
              <Icon
                name={stage.icon}
                size={16}
                className={stage.wave ? "text-wave" : "text-text-3"}
              />
            </div>

            <h3 className="mt-3 pl-4 text-title font-semibold tracking-[-0.015em] sm:pl-0">
              {stage.name}
            </h3>
            <p className="mt-2 max-w-[34ch] pl-4 text-caption leading-[1.6] text-text-2 sm:min-h-[4.2em] sm:pl-0 lg:min-h-[5.2em]">
              {stage.description}
            </p>
            <p className="mt-4 pl-4 font-mono text-[9px] leading-[1.6] tracking-[0.02em] text-text-3 sm:pl-0 lg:min-h-[2.6em]">
              {stage.output}
            </p>
          </li>
        ))}
      </ol>

      <div className="mt-6 flex flex-col gap-2 sm:flex-row sm:items-center sm:justify-between">
        <p className="text-caption text-text-3">
          Si una etapa falla, la app explica el motivo y continúa desde ahí.
        </p>
        <p className="font-mono text-[10px] tracking-[0.06em] text-text-3">
          Cada salida queda en la carpeta del proyecto.
        </p>
      </div>
    </div>
  );
}

import { Icon } from "@/components/ui/icon";
import { MonoLabel } from "@/components/ui/mono-label";

const words = [
  ["La"],
  ["ciu", "dad"],
  ["se"],
  ["en", "cien", "de"],
  ["y"],
  ["la"],
  ["no", "che"],
  ["em", "pie", "za"],
  ["a"],
  ["ha", "blar"],
];

const activeWord = 7;
const activeSyllable = 0;

const notes = [
  { x: 40, y: 150, w: 52 },
  { x: 100, y: 138, w: 40 },
  { x: 150, y: 126, w: 64 },
  { x: 224, y: 118, w: 36 },
  { x: 268, y: 110, w: 58 },
  { x: 336, y: 96, w: 44 },
  { x: 390, y: 120, w: 70 },
  { x: 470, y: 132, w: 38 },
  { x: 516, y: 112, w: 56 },
  { x: 582, y: 100, w: 72 },
  { x: 664, y: 118, w: 44 },
  { x: 716, y: 108, w: 60 },
  { x: 788, y: 122, w: 50 },
  { x: 846, y: 134, w: 66 },
  { x: 922, y: 118, w: 42 },
  { x: 974, y: 104, w: 58 },
  { x: 1044, y: 124, w: 70 },
  { x: 1124, y: 136, w: 44 },
];

const details = [
  {
    label: "01",
    title: "pYIN",
    body: "F0 por tramas, ataques y tempo. Las notas salen de la voz separada, no del instrumental.",
  },
  {
    label: "02",
    title: "Fonemas",
    body: "El alineador mide cada sílaba con IPA. Si falta espeak-ng, la letra se estima por líneas.",
  },
  {
    label: "03",
    title: "A mano",
    body: "Doble clic para editar una sílaba, arrastrar para seleccionar y F/R/N para cambiar el tipo.",
  },
];

export function EditorPreview() {
  return (
    <div>
      <figure className="border border-border bg-surface shadow-card">
        <div className="flex items-center justify-between gap-4 border-b border-border px-4 py-2.5">
          <div className="flex items-center gap-4 whitespace-nowrap">
            <span className="hidden text-caption text-text-3 sm:inline">
              Canción lista
            </span>
            <span className="border-b border-accent pb-0.5 text-caption text-text">
              Vista previa
            </span>
          </div>
          <MonoLabel className="whitespace-nowrap text-[9px] tracking-[0.14em]">
            <span className="sm:hidden">12 / 34</span>
            <span className="hidden sm:inline">Frase 12 / 34 · 9 s</span>
          </MonoLabel>
        </div>

        <div className="flex flex-wrap items-end justify-between gap-3 px-4 pt-5 pb-4 md:px-6">
          <div>
            <MonoLabel className="text-[9px] tracking-[0.16em]">
              Frase actual
            </MonoLabel>
            <p className="mt-2 text-caption text-text-3">
              Y la ciudad respira despacio
            </p>
            <p className="mt-2 flex flex-wrap items-baseline text-[19px] leading-[1.4] font-medium tracking-[-0.01em] md:text-[28px]">
              {words.map((word, wordIndex) => (
                <span
                  key={`${word.join("")}-${wordIndex}`}
                  className="mr-[0.3em] inline-block whitespace-nowrap"
                >
                  {word.map((syllable, syllableIndex) => {
                    const isActive =
                      wordIndex === activeWord &&
                      syllableIndex === activeSyllable;
                    return (
                      <span
                        key={`${syllable}-${syllableIndex}`}
                        className={
                          isActive
                            ? "border-b border-accent bg-accent/15 px-[0.5px] text-accent-text"
                            : "px-[0.5px] text-text"
                        }
                      >
                        {syllable}
                      </span>
                    );
                  })}
                </span>
              ))}
            </p>
          </div>
          <span className="hidden font-mono text-[10px] tracking-[0.04em] text-text-3 md:block">
            00:41.2 — 00:44.6
          </span>
        </div>

        <div className="relative border-y border-border bg-field">
          <svg
            viewBox="0 0 1200 220"
            preserveAspectRatio="none"
            aria-hidden="true"
            className="block h-[150px] w-full md:h-[190px]"
          >
            {Array.from({ length: 13 }, (_, index) => (
              <line
                key={`v-${index}`}
                x1={index * 100}
                x2={index * 100}
                y1={0}
                y2={220}
                stroke={
                  index % 4 === 0 ? "var(--grid-major)" : "var(--grid-minor)"
                }
                strokeWidth="1"
                vectorEffect="non-scaling-stroke"
              />
            ))}
            {[30, 70, 110, 150, 190].map((y) => (
              <line
                key={`h-${y}`}
                x1={0}
                x2={1200}
                y1={y}
                y2={y}
                stroke={y === 110 ? "var(--grid-major)" : "var(--grid-faint)"}
                strokeWidth="1"
                vectorEffect="non-scaling-stroke"
              />
            ))}

            {notes.map((note, index) => (
              <g key={`${note.x}-${note.y}`}>
                <rect
                  x={note.x}
                  y={note.y}
                  width={note.w}
                  height={12}
                  fill={
                    index === 11 ? "var(--accent)" : "var(--surface-4)"
                  }
                  stroke={
                    index === 11 ? "var(--accent-hover)" : "var(--border-hover)"
                  }
                  strokeWidth="1"
                  vectorEffect="non-scaling-stroke"
                />
                <line
                  x1={note.x + 3}
                  x2={note.x + 3}
                  y1={note.y + 12}
                  y2={note.y + 22}
                  stroke="var(--border)"
                  strokeWidth="1"
                  vectorEffect="non-scaling-stroke"
                />
              </g>
            ))}

            <line
              x1={748}
              x2={748}
              y1={0}
              y2={220}
              stroke="var(--accent)"
              strokeWidth="1"
              vectorEffect="non-scaling-stroke"
              opacity="0.55"
            />
            <rect x={743} y={0} width={10} height={3} fill="var(--accent)" />
          </svg>
        </div>

        <div className="flex items-center justify-between border-b border-border px-4 py-2">
          <div className="flex items-center gap-1">
            <span className="grid size-8 place-items-center text-text-3">
              <Icon name="skip-back" size={13} />
            </span>
            <span className="grid size-8 place-items-center bg-accent text-on-accent">
              <Icon name="play" size={13} />
            </span>
            <span className="grid size-8 place-items-center text-text-3">
              <Icon name="skip-forward" size={13} />
            </span>
          </div>

          <div className="flex items-center gap-4 font-mono text-[10px] text-text-3">
            <span className="text-text-2">00:42.318</span>
            <span className="border border-border px-1.5 py-0.5 text-text-2">
              1×
            </span>
            <span className="hidden items-center gap-1.5 sm:flex">
              <Icon name="minus" size={10} />
              <span>9 s</span>
              <Icon name="plus" size={10} />
            </span>
          </div>
        </div>

        <div className="grid grid-cols-5 px-4 py-2 font-mono text-[9px] tracking-[0.04em] text-text-3">
          {["41.0", "43.0", "45.0", "47.0", "49.0"].map((tick) => (
            <span key={tick} className="last:text-right">
              {tick}
            </span>
          ))}
        </div>

        <figcaption className="sr-only">
          Recreación del editor: una frase de letra con la sílaba activa
          resaltada y una lane de notas con la nota que suena en azul.
        </figcaption>
      </figure>

      <div className="mt-8 grid gap-6 border-t border-border pt-6 sm:grid-cols-3 sm:gap-8">
        {details.map((detail) => (
          <div key={detail.label}>
            <div className="flex items-baseline gap-2">
              <MonoLabel className="text-[9px] tracking-[0.16em]">
                {detail.label}
              </MonoLabel>
              <h3 className="text-body font-semibold">{detail.title}</h3>
            </div>
            <p className="mt-2 max-w-[38ch] text-caption leading-[1.65] text-text-2">
              {detail.body}
            </p>
          </div>
        ))}
      </div>
    </div>
  );
}

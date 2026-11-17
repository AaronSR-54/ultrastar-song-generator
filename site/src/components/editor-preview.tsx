import { AppShot } from "@/components/app-shot";
import { MonoLabel } from "@/components/ui/mono-label";

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
      <AppShot
        src="/app/editor.png"
        alt="Editor real de UltraStar Song Generator con la frase «love was such an easy game to play», las notas de la melodía, la sílaba activa y los controles de reproducción."
        label="App / Vista previa"
        caption="Captura real · The Beatles — Yesterday · 172 notas · 23 frases"
      />

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

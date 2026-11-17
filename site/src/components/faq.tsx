import { Icon } from "@/components/ui/icon";

const entries = [
  {
    question: "¿Funciona en mi Mac?",
    answer:
      "Hay binario para macOS 12 o superior con Apple Silicon (M1 en adelante). En un Mac Intel no hay ZIP ni DMG: el detector te lo dice y la tarjeta sigue visible con la opción de compilar con Rust 1.89+.",
  },
  {
    question: "Windows lo bloquea o macOS no lo abre.",
    answer:
      "Los binarios no están firmados. En Windows, SmartScreen muestra «Windows protegió tu PC»: pulsa Más información y después Ejecutar de todas formas. En macOS, haz clic derecho sobre la app y elige Abrir la primera vez; si sigue cerrada, revisa Ajustes del Sistema → Privacidad y seguridad → Abrir igualmente.",
  },
  {
    question: "Falta yt-dlp, ffmpeg o espeak-ng.",
    answer:
      "La app detecta las herramientas al arrancar y la etapa Descarga falla con un mensaje explícito antes de bajar nada. Instálalas, reinicia la terminal y comprueba con yt-dlp --version y ffmpeg -version. espeak-ng es opcional, pero sin él la letra se estima por líneas en vez de medirse por fonemas.",
  },
  {
    question: "¿Por qué descarga cientos de MB la primera vez?",
    answer:
      "Los pesos no van dentro del ZIP. El primer procesamiento descarga HTDemucs (~302 MB) y el alineador fonético (~318 MB); Whisper (~547 MB) solo si no hay letra online y eliges transcribir. Se guardan en ~/.local/share/usdx/models, %TEMP%\\usdx-models o junto al ejecutable; USDX_MODELS_DIR apunta a otra carpeta.",
  },
  {
    question: "¿En qué idioma canta?",
    answer:
      "La letra de LRCLIB trae el idioma; si falta, la app lo deduce del texto (español e inglés, además de otros alfabetos). USDX_LANGUAGE=es o en fuerza el idioma y el diccionario de silabificación. Con USDX_OFFLINE=1 no se busca letra online: la pegas o la transcribes.",
  },
  {
    question: "¿Puedo hacer duetos?",
    answer:
      "Todavía no: el MVP exporta una sola voz. El .txt es un UltraStar estándar y puedes volver a exportar desde Vista previa después de editar sílabas, tipos de nota o la carpeta de destino.",
  },
];

export function Faq() {
  return (
    <div className="border-t border-border">
      {entries.map((entry, index) => (
        <details key={entry.question} className="group border-b border-border">
          <summary className="flex cursor-pointer list-none items-center justify-between gap-6 py-5 text-[16px] font-medium tracking-[-0.01em] transition-colors duration-150 hover:text-accent-text md:text-[17px]">
            <span className="flex items-baseline gap-4">
              <span className="font-mono text-[9px] tracking-[0.16em] text-text-3">
                {String(index + 1).padStart(2, "0")}
              </span>
              {entry.question}
            </span>
            <span className="grid size-7 shrink-0 place-items-center border border-border text-text-3 transition-colors duration-150 group-hover:border-border-hover group-hover:text-text">
              <Icon name="plus" size={12} className="faq-plus" />
              <Icon name="minus" size={12} className="faq-minus" />
            </span>
          </summary>
          <p className="max-w-[74ch] pb-6 pl-8 text-body leading-[1.7] text-text-2">
            {entry.answer}
          </p>
        </details>
      ))}
    </div>
  );
}

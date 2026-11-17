import { Icon, type IconName } from "@/components/ui/icon";
import { MonoLabel } from "@/components/ui/mono-label";

type Platform = {
  id: string;
  icon: IconName;
  name: string;
  meta: string;
  command: string[];
  note: string;
};

const platforms: Platform[] = [
  {
    id: "windows",
    icon: "windows",
    name: "Windows",
    meta: "10 / 11 · x86_64",
    command: ["winget install yt-dlp.yt-dlp Gyan.FFmpeg OpenJS.NodeJS"],
    note: "SmartScreen puede pedir confirmación: Más información → Ejecutar de todas formas.",
  },
  {
    id: "macos",
    icon: "command",
    name: "macOS",
    meta: "12+ · Apple Silicon",
    command: ["brew install yt-dlp ffmpeg espeak-ng node"],
    note: "Gatekeeper: clic derecho → Abrir la primera vez. El binario no está notarizado.",
  },
  {
    id: "linux",
    icon: "terminal",
    name: "Linux",
    meta: "x86_64 · glibc",
    command: [
      "sudo apt install yt-dlp ffmpeg espeak-ng nodejs",
      "# Arch: sudo pacman -S yt-dlp ffmpeg espeak-ng nodejs",
    ],
    note: "Marca el binario como ejecutable: chmod +x usdx-desktop.",
  },
];

const required = [
  "yt-dlp 2025.09+ en el PATH",
  "Node.js ≥ 18 o Deno para yt-dlp",
  "ffmpeg y ffprobe 6.0+",
  "espeak-ng 1.50+ (opcional)",
];

export function Requirements() {
  return (
    <div>
      <div className="grid gap-px border border-border bg-border lg:grid-cols-3">
        {platforms.map((platform) => (
          <div key={platform.id} className="min-w-0 bg-bg p-5 md:p-6">
            <div className="flex items-start justify-between gap-3">
              <span className="grid size-9 place-items-center border border-border bg-surface-2 text-text-2">
                <Icon name={platform.icon} size={16} />
              </span>
              <MonoLabel className="pt-1 text-[9px] tracking-[0.14em]">
                {platform.meta}
              </MonoLabel>
            </div>

            <h3 className="mt-4 text-[19px] font-semibold tracking-[-0.02em]">
              {platform.name}
            </h3>

            <ul className="mt-4 space-y-2">
              {required.map((item) => (
                <li
                  key={item}
                  className="flex items-start gap-2 text-caption leading-[1.5] text-text-2"
                >
                  <Icon
                    name="check"
                    size={11}
                    className="mt-1 shrink-0 text-text-3"
                  />
                  {item}
                </li>
              ))}
            </ul>

            <div className="mt-5 border border-border bg-field p-3">
              <pre className="font-mono text-[10px] leading-[1.8] whitespace-pre-wrap text-text-2 select-all">
                {platform.command.map((line) => (
                  <div
                    key={line}
                    className={line.startsWith("#") ? "text-text-3" : undefined}
                  >
                    {line}
                  </div>
                ))}
              </pre>
            </div>

            <p className="mt-4 flex gap-2 text-caption leading-[1.6] text-text-3">
              <Icon
                name="alert"
                size={11}
                className="mt-1 shrink-0 text-warning"
              />
              {platform.note}
            </p>
          </div>
        ))}
      </div>

      <div className="mt-6 grid gap-6 border-t border-border pt-6 sm:grid-cols-2">
        <div>
          <MonoLabel className="text-[9px] tracking-[0.16em]">
            Modelos · primer uso
          </MonoLabel>
          <p className="mt-2 max-w-[52ch] text-caption leading-[1.65] text-text-2">
            ~620 MB: HTDemucs y el alineador fonético se descargan solos. Whisper
            (~547 MB) solo si no hay letra online y eliges transcribir.
          </p>
        </div>
        <div>
          <MonoLabel className="text-[9px] tracking-[0.16em]">
            Compilar · opcional
          </MonoLabel>
          <p className="mt-2 max-w-[52ch] text-caption leading-[1.65] text-text-2">
            Los ZIP son binarios. Solo necesitas Rust 1.89+ si compilas desde el
            código; los modelos no van dentro del ejecutable.
          </p>
        </div>
      </div>
    </div>
  );
}

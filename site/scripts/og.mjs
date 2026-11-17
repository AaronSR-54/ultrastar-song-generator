import { mkdir } from "node:fs/promises";
import { resolve } from "node:path";

import sharp from "sharp";

const sans = "DejaVu Sans, Segoe UI, system-ui, sans-serif";
const mono = "DejaVu Sans Mono, Consolas, monospace";

const svg = `<svg xmlns="http://www.w3.org/2000/svg" width="1200" height="630" viewBox="0 0 1200 630">
  <defs>
    <pattern id="dots" width="32" height="32" patternUnits="userSpaceOnUse">
      <circle cx="1" cy="1" r="1.1" fill="#8A8C97" fill-opacity="0.13" />
    </pattern>
  </defs>

  <rect width="1200" height="630" fill="#0A0A0C" />
  <rect width="1200" height="630" fill="url(#dots)" />
  <rect x="32" y="32" width="1136" height="566" fill="none" stroke="#2A2C36" />

  <rect x="80" y="64" width="40" height="40" fill="#4F8DFF" fill-opacity="0.15" stroke="#4F8DFF" stroke-opacity="0.55" />
  <g transform="translate(88 72) scale(1.333)">
    <path d="M9 18V5.6l10-2v12.8" fill="none" stroke="#9DC0FF" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" />
    <circle cx="6.4" cy="18" r="2.6" fill="none" stroke="#9DC0FF" stroke-width="1.5" />
    <circle cx="16.4" cy="16.4" r="2.6" fill="none" stroke="#9DC0FF" stroke-width="1.5" />
  </g>

  <text x="136" y="82" font-family="${sans}" font-size="19" font-weight="600" fill="#F4F4F7">UltraStar Song Generator</text>
  <text x="136" y="101" font-family="${mono}" font-size="11" letter-spacing="2" fill="#82848F">USDX // LOCAL PIPELINE</text>

  <text x="80" y="238" font-family="${sans}" font-size="76" font-weight="600" letter-spacing="-3" fill="#F4F4F7">De un enlace</text>
  <text x="80" y="318" font-family="${sans}" font-size="76" font-weight="600" letter-spacing="-3" fill="#F4F4F7">de YouTube a una</text>
  <text x="80" y="398" font-family="${sans}" font-size="76" font-weight="600" letter-spacing="-3" fill="#F4F4F7">canción jugable.</text>

  <rect x="80" y="444" width="64" height="3" fill="#4F8DFF" />

  <text x="80" y="496" font-family="${sans}" font-size="24" fill="#A0A2AD">Letra, voz y pitch en tu equipo.</text>
  <text x="80" y="530" font-family="${sans}" font-size="24" fill="#A0A2AD">Un .txt de UltraStar con audio, carátula y vídeo.</text>

  <text x="80" y="576" font-family="${mono}" font-size="13" letter-spacing="1" fill="#82848F">WINDOWS · MACOS · LINUX</text>
  <text x="1120" y="576" text-anchor="end" font-family="${mono}" font-size="13" letter-spacing="0.5" fill="#82848F">github.com/AaronSR-54/ultrastar-song-generator</text>

  <g transform="translate(828 150)">
    <rect width="292" height="330" fill="#15161B" stroke="#2A2C36" />
    <rect width="292" height="30" fill="#1B1C22" />
    <line x1="0" y1="30" x2="292" y2="30" stroke="#2A2C36" />
    <rect x="10" y="9" width="12" height="12" fill="#4F8DFF" fill-opacity="0.16" stroke="#4F8DFF" stroke-opacity="0.5" />
    <text x="30" y="20" font-family="${sans}" font-size="10" font-weight="600" fill="#F4F4F7">UltraStar Song Generator</text>
    <text x="242" y="20" font-family="${mono}" font-size="9" fill="#82848F">— □ ×</text>

    <text x="22" y="62" font-family="${mono}" font-size="8" letter-spacing="1.5" fill="#82848F">INICIO</text>
    <text x="22" y="94" font-family="${sans}" font-size="21" font-weight="600" fill="#F4F4F7">Nueva canción</text>
    <text x="22" y="114" font-family="${sans}" font-size="10" fill="#A0A2AD">Pega el enlace. La app descarga,</text>
    <text x="22" y="129" font-family="${sans}" font-size="10" fill="#A0A2AD">separa, mide y exporta.</text>

    <text x="22" y="162" font-family="${mono}" font-size="8" letter-spacing="1" fill="#82848F">URL DEL VÍDEO</text>
    <rect x="22" y="172" width="248" height="34" fill="#0D0D11" stroke="#2A2C36" />
    <text x="32" y="194" font-family="${mono}" font-size="9" fill="#82848F">https://www.youtube.com/watch?v=…</text>

    <rect x="22" y="216" width="248" height="34" fill="#4F8DFF" />
    <text x="146" y="238" text-anchor="middle" font-family="${sans}" font-size="11" font-weight="600" fill="#080D18">Generar →</text>

    <line x1="22" y1="270" x2="270" y2="270" stroke="#2A2C36" />
    <text x="22" y="290" font-family="${mono}" font-size="8" fill="#82848F">yt-dlp 2026.10.21</text>
    <text x="270" y="290" text-anchor="end" font-family="${mono}" font-size="8" fill="#82848F">WAV 48 kHz</text>
    <text x="22" y="312" font-family="${mono}" font-size="8" fill="#82848F">local · sin cuentas</text>
    <text x="270" y="312" text-anchor="end" font-family="${mono}" font-size="8" fill="#82848F">0 subidas</text>
  </g>
</svg>`;

const target = resolve(process.cwd(), "public/og.png");
await mkdir(resolve(process.cwd(), "public"), { recursive: true });

const info = await sharp(Buffer.from(svg))
  .png({ compressionLevel: 9 })
  .toFile(target);

console.log(`og.png ${info.width}×${info.height}, ${Math.round(info.size / 1024)} KB`);

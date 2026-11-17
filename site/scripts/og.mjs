import { mkdir, readFile } from "node:fs/promises";
import { resolve } from "node:path";

import sharp from "sharp";

const sans = "DejaVu Sans, Segoe UI, system-ui, sans-serif";
const mono = "DejaVu Sans Mono, Consolas, monospace";

const shot = await readFile(resolve(process.cwd(), "public/app/home.png"));
const shotUri = `data:image/png;base64,${shot.toString("base64")}`;

const feather =
  "M14.27 12.62 20.32 7.36 20.08 7.04 13.33 11.38ZM10.67 11.38 3.92 7.04 3.68 7.36 9.73 12.62ZM14.51 14.32 21.18 10.78 21.02 10.42 13.89 12.88ZM10.11 12.88 2.98 10.42 2.82 10.78 9.49 14.32ZM13.96 15.98 19.92 14.9 19.88 14.5 13.84 14.42ZM10.16 14.42 4.12 14.5 4.08 14.9 10.04 15.98Z";

const svg = `<svg xmlns="http://www.w3.org/2000/svg" width="1200" height="630" viewBox="0 0 1200 630">
  <defs>
    <pattern id="dots" width="32" height="32" patternUnits="userSpaceOnUse">
      <circle cx="1" cy="1" r="1.1" fill="#8A8C97" fill-opacity="0.13" />
    </pattern>
  </defs>

  <rect width="1200" height="630" fill="#0A0A0C" />
  <rect width="1200" height="630" fill="url(#dots)" />
  <rect x="32" y="32" width="1136" height="566" fill="none" stroke="#2A2C36" />

  <rect x="80" y="64" width="42" height="42" fill="#4F8DFF" fill-opacity="0.15" stroke="#4F8DFF" stroke-opacity="0.45" />
  <g transform="translate(88 72) scale(1.0833)">
    <path d="${feather}" fill="#9DC0FF" fill-opacity="0.55" />
    <circle cx="12" cy="6.6" r="2.9" fill="none" stroke="#9DC0FF" stroke-width="1.7" />
    <g stroke="#9DC0FF" stroke-width="0.85" stroke-linecap="round">
      <path d="M10.7 5.2h2.6M10.4 6.6h3.2M10.7 8h2.6" />
    </g>
    <path d="M9.9 9.3h4.2v1.2H9.9ZM10.7 10.5h2.6l-.3 8.5h-2Z" fill="#9DC0FF" />
  </g>

  <text x="138" y="84" font-family="${sans}" font-size="19" font-weight="600" fill="#F4F4F7">UltraStar Song Generator</text>
  <text x="138" y="103" font-family="${mono}" font-size="11" letter-spacing="2" fill="#82848F">USDX // LOCAL PIPELINE</text>

  <text x="80" y="246" font-family="${sans}" font-size="76" font-weight="600" letter-spacing="-3" fill="#F4F4F7">De un enlace</text>
  <text x="80" y="326" font-family="${sans}" font-size="76" font-weight="600" letter-spacing="-3" fill="#F4F4F7">de YouTube a una</text>
  <text x="80" y="406" font-family="${sans}" font-size="76" font-weight="600" letter-spacing="-3" fill="#F4F4F7">canción jugable.</text>

  <rect x="80" y="452" width="64" height="3" fill="#4F8DFF" />

  <text x="80" y="504" font-family="${sans}" font-size="24" fill="#A0A2AD">Letra, voz y pitch en tu equipo.</text>
  <text x="80" y="538" font-family="${sans}" font-size="24" fill="#A0A2AD">Un .txt de UltraStar con audio, carátula y vídeo.</text>

  <text x="80" y="580" font-family="${mono}" font-size="13" letter-spacing="1" fill="#82848F">WINDOWS · MACOS · LINUX</text>

  <image x="690" y="166" width="440" height="296" href="${shotUri}" />
  <rect x="689" y="165" width="442" height="298" fill="none" stroke="#2A2C36" />
  <text x="690" y="500" font-family="${mono}" font-size="11" letter-spacing="1.5" fill="#82848F">CAPTURA REAL · INICIO</text>
  <text x="1130" y="500" text-anchor="end" font-family="${mono}" font-size="11" letter-spacing="0.5" fill="#82848F">v0.1.0</text>

  <text x="690" y="536" font-family="${sans}" font-size="16" fill="#F4F4F7">Una URL. Seis etapas. Cero subidas.</text>
  <text x="690" y="562" font-family="${mono}" font-size="11" fill="#82848F">github.com/AaronSR-54/ultrastar-song-generator</text>
</svg>`;

const target = resolve(process.cwd(), "public/og.png");
await mkdir(resolve(process.cwd(), "public"), { recursive: true });

const info = await sharp(Buffer.from(svg))
  .png({ compressionLevel: 9 })
  .toFile(target);

console.log(`og.png ${info.width}×${info.height}, ${Math.round(info.size / 1024)} KB`);

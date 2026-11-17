# Design

## Context

- La app es un binario Rust/egui (`crates/desktop`). Su lenguaje visual vive en `crates/desktop/src/theme/`: paleta única, `RADIUS = 0`, retícula de puntos, seis pasos tipográficos, cinco pasos de espaciado, iconos vectoriales de 24 × 24, hover de 140 ms y pulsación de 70 ms.
- `.github/workflows/release.yml` hoy compila las tres plataformas y sube artefactos internos de Actions. No crea releases, no hay tags y los binarios no son descargables públicamente.
- `openspec/specs/` está vacío: no hay comportamiento previo que preservar fuera de la app.
- El destino elegido es Vercel con `site/` como *root directory*. `site/` queda fuera del workspace Cargo.
- Ver `proposal.md` para la motivación y `specs/landing-page/spec.md` + `specs/app-distribution/spec.md` para el contrato de comportamiento.

## Goals / Non-Goals

**Goals:**

- Landing de una sola página, en español, que se sienta como la app y no como una plantilla: composición propia, jerarquía clara y detalle técnico.
- Fidelidad de tokens: mismos colores, esquinas rectas, retícula, tipografía de sistema, micro-etiquetas mono y un único acento.
- Descarga correcta por sistema y arquitectura, sin bloquear otras plataformas y sin depender de JavaScript para ver las opciones.
- Releases automáticas con nombres de asset estables y sumas SHA-256.
- Página estática/ISR con un solo componente cliente; sin fuentes web, sin imágenes raster en el cuerpo y sin librerías de animación.

**Non-Goals:**

- Firmar o notarizar binarios, instalar la app desde una store, auto-update ni empaquetar `.app`/`.dmg`/`.msi`.
- Documentación completa del producto (sigue en `README.md` y `docs/`).
- Internacionalización, blog, changelog propio, analytics o formularios.
- Sustituir el README como guía de compilación.
- Cambios en el pipeline o en el formato de exportación.

## Decisions

### 1. Next.js 16 + TypeScript + Tailwind CSS 4 en `site/`

- **App Router**, React 19, `output` por defecto (Vercel), render en servidor del contenido y un único *island* cliente para la detección.
- **Tailwind 4 CSS-first**: `@import "tailwindcss"` y `@theme inline` mapeando los tokens a utilidades (`bg-bg`, `text-text-2`, `border-border`, `bg-surface`, `text-accent`). No hay `tailwind.config`.
- **npm con `package-lock.json`**, Node 22 en CI y en Vercel.
- Alternativas: Vite + React (más configuración propia y sin ventajas en Vercel), Astro (excelente para estático, pero el usuario pidió Next y el panel de descarga necesita React), HTML plano (difícil de mantener el sistema de componentes).
- **Sin fuentes web**: `font-family` de sistema (`system-ui`, `-apple-system`, `Segoe UI`, `Roboto`, `DejaVu Sans`) y mono de sistema. Coincide con la app, elimina FOUT/CLS y ahorra ~50–100 KB.

### 2. Espejo del sistema de diseño

`site/src/styles/tokens.css` declara las variables y Tailwind las expone:

| Token | Valor |
|---|---|
| `--bg` | `#0A0A0C` |
| `--surface` / `--surface-2` / `--surface-3` | `#15161B` / `#1B1C22` / `#23242B` |
| `--field` | `#0D0D11` |
| `--border` / `--border-hover` | `#2A2C36` / `#3C3E48` |
| `--text` / `--text-2` / `--text-3` | `#F4F4F7` / `#A0A2AD` / `#82848F` |
| `--accent` / `--accent-hover` / `--accent-pressed` | `#4F8DFF` / `#6B9FFF` / `#3E77E6` |
| `--accent-text` / `--on-accent` | `#9DC0FF` / `#080D18` |
| `--wave` | `#6EA8FF` |
| `--success` / `--warning` / `--danger` | `#3DD68C` / `#F2C14E` / `#FF5F5F` |
| `--radius` | `0` |

- **Escala de UI** igual a la app: micro 11, caption 12.5, body 14, title 16, display 20, hero 24. La landing añade **solo** tres tamaños de marketing: `--text-display-lg` 40, `--text-display-xl` 64 y `--text-hero-lg` `clamp(40px, 7.2vw, 88px)`, con la misma familia y peso semibold.
- **Espaciado**: 4/8/12/16/24 de la app para componentes; en secciones se usan múltiplos (`48`, `64`, `96`, `128`) para ritmo vertical.
- **Elevación**: una sola sombra `0 8px 24px rgb(0 0 0 / 0.43)`; los modales de la landing no existen.
- **Iconos**: SVG inline dibujados a mano sobre rejilla de 24, trazo 1.6–2 y `currentColor`, replicando el estilo de `crates/desktop/src/icons.rs`. No se instala una librería de iconos.

### 3. Dirección de arte (requisito de calidad)

La landing debe leerse como una pieza de diseño, no como una plantilla de SaaS.

**Composición**

- Ancho máximo de contenido `1200px`; hero asimétrico en 12 columnas: 6 de texto, 6 de ventana de app (a 1024 px pasa a 5/7 y a 768 px se apila).
- Líneas de 1 px separan secciones; cada sección abre con un índice mono (`01 — CÓMO FUNCIONA`) y un `h2` grande. Nada de tarjetas flotantes idénticas en rejilla de tres.
- El fondo es plano (`--bg`) con retícula de puntos de 32 px al 7 % de opacidad y un degradado radial muy tenue solo como viñeta; nunca degradados de color.
- Alternancia de ritmo: bloque ancho (hero), bloque de 6 columnas (pipeline), bloque partido 50/50 (privacidad), bloque visual grande (editor), bloque de 3 columnas con hairlines (requisitos), bloque de descargas, FAQ, footer.

**Tipografía**

- `h1` semibold, `letter-spacing: -0.035em`, `line-height: 0.98`, máximo 14 palabras.
- Cuerpo 16/1.65 con medida de 62–72 caracteres; nunca párrafos de más de 3 líneas en columnas.
- Micro-etiquetas en mayúsculas, 11 px, mono, `letter-spacing: 0.08em`, color `--text-3`.
- Cifras y comandos en mono con `font-variant-numeric: tabular-nums`.

**Color**

- El acento se usa solo en la llamada principal, el estado activo del selector, el chip «Tu sistema» y el punto de reproducción del mock. Todo lo demás es neutral.
- `success`/`warning`/`danger` solo para estado de plataforma y avisos, nunca como decoración.
- Prohibido: morado, degradados de marca, glassmorphism, `border-radius` en botones/tarjetas, emojis, blobs, sombras de color, ilustraciones genéricas de stock.

**Piezas visuales**

- **Ventana de app** reconstruida en HTML/CSS: barra de título con marca, `USDX`, tres botones; pantalla de inicio con logo de 48 px, titular, campo de URL y botón «Generar»; borde de 1 px y sombra de elevación. Es el ancla visual del hero y demuestra el producto sin capturas desactualizadas.
- **Pipeline**: seis estaciones con número mono, icono y una línea. Conectores de 1 px; la etapa «Align» se marca con `--wave` como única referencia al audio.
- **Editor**: recreación de la franja de letra y la lane de notas con grid (`--grid-major`/`--grid-minor`), bloques de nota y una nota activa en `--accent`. SVG con etiquetas `aria-hidden` y texto alternativo en el contenedor.
- **Privacidad**: diagrama de tres columnas «YouTube → tu equipo → tu carpeta» con una flecha de red marcada solo para metadatos; el resto en local.
- **Requisitos**: tabla/columnas con checks y comandos copiables en mono.

**Detalle fino**

- Hairlines de 1 px que se alinean con la retícula; nunca bordes de 2 px.
- Botones de 52 px de alto en la landing (44 px en densidades compactas), padding 24, etiqueta semibold 14–16 y desplazamiento del icono de 1.5 px en hover, igual que la app.
- Foco: anillo de 1 px `--accent` con `outline-offset: 3px`, nunca `box-shadow` de color.
- Estados vacíos y de error con icono vectorial + texto, como `error_row` de la app.

**Proceso de calidad** (obligatorio en implementación): capturas reales en 360, 768, 1024 y 1440 px; revisión contra la lista anterior; corregir y repetir hasta que no haya desalineaciones, texto huérfano ni secciones planas. La aceptación incluye Lighthouse móvil ≥ 95/100/100/100.

### 4. Mapa de la página

1. **Cabecera fija** (`sticky`, fondo `--bg` al 95 %, hairline inferior): marca + `UltraStar // Song Generator`, anclas «Cómo funciona», «Requisitos», «Descargas», botón compacto de descarga y enlace a GitHub.
2. **Hero**: eyebrow mono con versión y plataformas, `h1` «De un enlace de YouTube a una canción jugable.», subtítulo de dos líneas, llamada principal OS-aware, secundaria «Ver el código», línea de confianza («local, sin subir tu audio»), ventana de app.
3. **01 — Cómo funciona**: pipeline de seis etapas.
4. **02 — Todo ocurre en tu equipo**: privacidad + diagrama; tres puntos (Demucs local, Whisper local, solo metadatos a LRCLIB).
5. **03 — La letra, medida**: editor con notas y frase actual; copy sobre pYIN, fonemas y corrección manual.
6. **04 — Lista para UltraStar**: árbol de exportación en panel inset, fragmento `.txt` y detección de `SongDir1`.
7. **05 — Requisitos**: tres columnas por plataforma con herramientas y avisos.
8. **06 — Descargas**: selector de plataforma y tarjetas con ZIP, arquitectura, SHA-256 y nota de instalación; enlace a `SHA256SUMS.txt` y comando de verificación.
9. **FAQ**: seis preguntas en `<details>` nativos con icono `+`/`−`.
10. **Footer**: marca, licencia MIT OR Apache-2.0, enlaces a repo/docs/releases y micro-línea «Hecho en local. Como la app.».

### 5. Detección de plataforma y datos de descarga

- `src/lib/platforms.ts` expone una función pura `detectPlatform(input)` con `input = { userAgent, uaData?: { platform, architecture, bitness }, maxTouchPoints }`. Devuelve `{ os, arch, confidence, asset?, reason? }`.
- **Client Hints primero**: `navigator.userAgentData.getHighEntropyValues(["architecture", "bitness", "platformVersion"])`; si falla o tarda, se usa el user agent con confianza `low`.
- **Reglas**:
  - `Windows` + `x86`/`x86_64` → `windows-x86_64`; `Windows` + `arm` → sin binario.
  - `macOS`/`MacIntel` + `arm` → `macos-aarch64`; `MacIntel` con `maxTouchPoints > 1` → iPadOS, sin binario; `macOS` Intel (solo Chromium lo distingue) → sin binario con nota de compilación.
  - `Linux` + `x86_64` → `linux-x86_64`; `arm64`/`aarch64` → sin binario.
  - `Android`, `iOS`, desconocido → sin binario; se muestran las tres tarjetas sin recomendación.
- **Render sin hydration mismatch**: el Server Component pinta las tres tarjetas y la llamada neutral «Descargar». `DownloadPanel` (cliente) arranca con `detected = null`, detecta en `useEffect` y solo entonces cambia la etiqueta y el chip «Tu sistema». El espacio del chip y de la nota está reservado para no provocar CLS.
- **Selección manual**: tres `input[type=radio]` visualmente convertidos en tarjetas (accesible por teclado sin JS extra). Cambiar de plataforma actualiza la llamada principal y anuncia el cambio con `aria-live="polite"`.
- **Sin JavaScript**: los tres enlaces directos y sus etiquetas ya están en el HTML; el selector no es imprescindible.
- **Datos de release**: `src/lib/downloads.ts` (servidor) consulta `api.github.com/repos/AaronSR-54/ultrastar-song-generator/releases/latest` con `next: { revalidate: 300 }`, `Accept: application/vnd.github+json` y `GITHUB_TOKEN` opcional (eleva el límite de 60 a 5000 peticiones/hora). Devuelve `{ version, publishedAt, assets }` o `unavailable`.
  - Si hay release: enlaces directos a `releases/latest/download/<asset>` (URLs estables) y se muestra la versión detectada.
  - Si la API responde sin los assets esperados: estado vacío «Todavía no hay binarios publicados» + enlace a Actions y a compilar.
  - Si la API falla: las tarjetas enlazan a la página de releases (no a un asset que podría devolver 404) con la nota «No hemos podido comprobar la última versión».
- **Nombres de asset** (contrato compartido con el workflow): `usdx-desktop-windows-x86_64.zip`, `usdx-desktop-macos-aarch64.zip`, `usdx-desktop-linux-x86_64.zip`, `SHA256SUMS.txt`.

### 6. Releases automáticas

- `.github/workflows/release.yml` conserva el job `build` y añade `publish` con `needs: build`, `if: startsWith(github.ref, 'refs/tags/')` y `permissions: { contents: write }`.
- `publish` corre en `ubuntu-latest`: descarga los tres artefactos, normaliza el nombre (`zip -j`), da `chmod +x` a los binarios Unix antes de comprimir, genera `SHA256SUMS.txt` con las sumas de los tres ZIP y publica con `softprops/action-gh-release@v2` (`generate_release_notes: true`, `fail_on_unmatched_files: true`). El ZIP conserva el bit de ejecución.
- El workflow manual (`workflow_dispatch`) sigue generando artefactos, pero no publica: la release queda ligada a un tag.
- Los nombres de asset no incluyen versión para que `releases/latest/download/<asset>` sea estable entre versiones.
- Verificación en CI: tras comprimir, `unzip` en un directorio temporal y comprobar que `usdx-desktop` es ejecutable; `sha256sum -c` sobre el archivo generado.

### 7. CI del sitio

- `.github/workflows/site.yml` con `on: pull_request` y `push` filtrado por `paths: ["site/**", ".github/workflows/site.yml"]`.
- Pasos: `actions/setup-node@v4` (Node 22, cache npm) → `npm ci` → `npm run lint` → `npm run typecheck` → `npm run test` → `npm run build`.
- `concurrency` para cancelar ejecuciones antiguas. Sin despliegue desde Actions: Vercel construye con su integración Git.

### 8. Estructura del sitio

```
site/
  app/
    layout.tsx           # lang="es", metadata, viewport, JSON-LD SoftwareApplication
    page.tsx             # composición (Server Component)
    globals.css          # tokens + base + utilidades propias
    icon.svg
  src/
    components/
      site-header.tsx
      hero.tsx
      app-window.tsx
      download-panel.tsx      # "use client"
      platform-card.tsx
      checksum-note.tsx
      pipeline.tsx
      privacy.tsx
      editor-preview.tsx
      export-preview.tsx
      requirements.tsx
      faq.tsx
      site-footer.tsx
      ui/button.tsx
      ui/mono-label.tsx
      ui/section.tsx
      ui/icon.tsx
    lib/
      platforms.ts
      downloads.ts
      cn.ts
    styles/tokens.css
  public/
    og.png                 # 1200 × 630, generado desde scripts/og.mjs
  scripts/og.mjs
  tests/platforms.test.ts
  tests/assets.test.ts
  README.md
  package.json
  tsconfig.json
  eslint.config.mjs
  postcss.config.mjs
  next.config.ts
```

- `page.tsx` es el único punto que obtiene datos de release y los pasa a `DownloadPanel`; el resto es estático.
- `tests/assets.test.ts` lee `../.github/workflows/release.yml` y falla si un nombre de asset de `downloads.ts` no aparece en el workflow, evitando que landing y release se desincronicen.
- La imagen Open Graph se genera una vez con `sharp` desde una plantilla SVG (misma retícula, logo y titular) y se versiona en `public/og.png`; no se usa Satori ni fuentes embebidas en runtime.

### 9. Movimiento

- CSS puro. Hover 140 ms `ease-out`, pulsación 70 ms con `transform: scale(0.99)`, aparición al hacer scroll 480 ms `cubic-bezier(0.22, 1, 0.36, 1)` con `IntersectionObserver` y retardo escalonado de 40 ms.
- Nada de parallax, scroll-jacking ni animaciones en bucle. La ventana de app y el editor son estáticos; el movimiento solo confirma interacción.
- `@media (prefers-reduced-motion: reduce)` desactiva transiciones, aparición y `scroll-behavior: smooth`.

### 10. Copia y metadatos

- Tono: frases cortas, sin signos de exclamación, sin superlativos vacíos. Español de España, coherente con el README.
- `metadata`: título «UltraStar Song Generator — de YouTube a canción jugable», descripción de 150–160 caracteres, `metadataBase` de Vercel, `openGraph`/`twitter` con `og.png`, `lang="es"`, `robots: index, follow`.
- JSON-LD `SoftwareApplication` con `operatingSystem`, `applicationCategory: MultimediaApplication`, `offers: { price: 0 }` y enlace de descarga.
- Todos los enlaces externos con `rel="noreferrer noopener"` y texto que describe el destino.

## Risks / Trade-offs

- **Detección imperfecta en Safari (macOS Intel vs Apple Silicon)** → se recomienda Apple Silicon, se etiqueta la tarjeta como «Mac con Apple Silicon» y se ofrece la compilación desde código; el selector manual siempre está visible.
- **Límite de la API de GitHub (60/h por IP sin token)** → ISR de 300 s, `GITHUB_TOKEN` opcional en Vercel y estado de respaldo que enlaza a la página de releases.
- **Binarios sin firma** → SmartScreen y Gatekeeper bloquearán la primera ejecución; se documenta el paso exacto por plataforma y se ofrecen checksums. La notarización queda fuera de alcance.
- **Nombres de asset estables** → si una release antigua se reutiliza, `latest` siempre apunta a la más reciente; el workflow exige `fail_on_unmatched_files` y un job de publicación único.
- **Deriva de contenido** → la copia se limita a funciones presentes en `README.md`/`docs/`; la tarea de revisión final compara cada afirmación.
- **Beauty vs. tokens** → la landing amplía la escala tipográfica y el espaciado, pero no introduce colores, radios ni fuentes nuevas; la lista de prohibiciones de la dirección de arte es parte de la aceptación.
- **Tailwind 4 + Next 16 recientes** → versiones fijadas en `package-lock.json` y CI que construye en cada PR.
- **Peso del ZIP y expectativas** → los modelos (~620 MB) se descargan en el primer uso, no en el ZIP; la sección de requisitos lo explica antes de descargar.

## Migration Plan

1. Aterrizar `site/` y los workflows; ejecutar CI del sitio en verde.
2. Etiquetar `v0.1.0` para que el job `publish` cree la primera release con los cuatro assets.
3. Importar el repositorio en Vercel con `site/` como root, Node 22 y `npm ci`; configurar `GITHUB_TOKEN` opcional para la API.
4. Verificar las tres URLs `releases/latest/download/*`, el `SHA256SUMS.txt` y la detección en Windows, macOS y Linux reales o emulados.
5. Rollback: Vercel permite volver a un despliegue anterior; borrar la release o el tag revierte la distribución pública sin tocar la app.

## Open Questions

- Dominio final (subdominio de Vercel o dominio propio): no cambia el diseño ni los artefactos; se decide al conectar Vercel.
- Analítica de descargas: aplazada; si se añade, debe ser sin cookies y sin bloquear el render.

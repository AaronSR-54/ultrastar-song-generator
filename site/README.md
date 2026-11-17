# Landing de UltraStar Song Generator

Sitio público de producto y descargas, construido con Next.js 16 (App Router), React 19, TypeScript y Tailwind CSS 4. Reutiliza el lenguaje visual de la app de escritorio: paleta oscura, acento azul único, esquinas rectas, retícula de puntos y tipografía de sistema.

## Requisitos

- Node.js 22 o superior (probado con 22 y 26).
- npm 10 o superior.
- No necesita Rust, ni el workspace Cargo, ni variables de entorno obligatorias.

## Comandos

```bash
cd site
npm ci
npm run dev        # http://localhost:3000
npm run lint
npm run typecheck
npm run test
npm run build
npm start          # sirve el build de producción
```

## Estructura

```
site/
  app/                 # layout, página, estilos globales e icono
  src/components/      # secciones y primitivas de UI
  src/lib/             # detección de plataforma y datos de release
  src/styles/          # tokens del sistema de diseño
  public/              # og.png y estáticos
  scripts/             # generación de la imagen Open Graph
  tests/               # Vitest (detección y contrato de assets)
```

## Despliegue en Vercel

Producción: <https://ultrastar-song-generator.vercel.app>

El proyecto está conectado al repositorio de GitHub con **Root Directory** en `site`, Node.js 22.x, Install Command `npm ci` y Build Command `npm run build`. Los despliegues los hace la integración Git de Vercel; no hay despliegue desde GitHub Actions.

Para recrearlo desde cero:

1. Importa `AaronSR-54/ultrastar-song-generator` en Vercel.
2. Configura **Root Directory** en `site`.
3. Framework: Next.js. Node.js: 22.x. Install Command: `npm ci`. Build Command: `npm run build`.
4. No hay variables obligatorias. Opcionalmente define `GITHUB_TOKEN` (token de solo lectura de repositorio público) para elevar el límite de la API de GitHub que consulta la última release; sin él, el sitio funciona con el límite anónimo y un estado de respaldo.
5. El dominio final puede ser el subdominio de Vercel o uno propio; no requiere cambios de código.

Vercel construye con su integración Git. No hay despliegue desde GitHub Actions.

## Contrato de descargas

Los enlaces usan la URL estable:

```
https://github.com/AaronSR-54/ultrastar-song-generator/releases/latest/download/<asset>
```

Assets que publica `.github/workflows/release.yml` en cada tag `v*`:

| Asset | Plataforma |
|---|---|
| `usdx-desktop-windows-x86_64.zip` | Windows 10/11 x86_64 |
| `usdx-desktop-macos-aarch64.zip` | macOS Apple Silicon (arm64) |
| `usdx-desktop-linux-x86_64.zip` | Linux x86_64 |
| `SHA256SUMS.txt` | Sumas SHA-256 de los tres ZIP |

Los nombres no incluyen versión: `releases/latest` siempre apunta a la última. Un test (`tests/assets.test.ts`) comprueba que cada asset de `src/lib/platforms.ts` aparece literalmente en el workflow de release.

La release también adjunta los instaladores nativos (`.msi`, `.dmg`, `.deb`) con su propio `SHA256SUMS-installers.txt`; la landing usa solo los ZIP estables, porque funcionan en las tres plataformas sin instalador.

## Publicar una versión

1. Etiqueta el commit con una versión semántica y empuja el tag:

   ```bash
   git tag v0.1.0
   git push origin v0.1.0
   ```

2. El workflow `Release` compila y prueba las tres plataformas; solo si todas pasan, el job `publish` crea el ZIP de cada binario, genera `SHA256SUMS.txt` y publica los cuatro assets.
3. `workflow_dispatch` compila y sube artefactos de CI, pero no publica ninguna release: la publicación está ligada a un tag `v*`.
4. Comprueba las URLs estables:

   ```bash
   curl -I https://github.com/AaronSR-54/ultrastar-song-generator/releases/latest/download/usdx-desktop-linux-x86_64.zip
   ```

Si cambias un nombre de asset, actualiza `src/lib/platforms.ts` y vuelve a ejecutar `npm run test`: el test de contrato falla hasta que coincidan landing y workflow.

## Criterios de calidad

- Un solo componente cliente (`DownloadPanel`); el resto se renderiza en servidor.
- Sin fuentes web, sin librerías de iconos, sin imágenes raster en el cuerpo.
- La detección de plataforma nunca esconde las otras descargas y el HTML inicial contiene las tres.
- Tokens verificados por `tests/tokens.test.ts` contra `crates/desktop/src/theme/palette.rs`.
- Objetivo Lighthouse móvil: Rendimiento ≥ 95, Accesibilidad/Buenas prácticas/SEO = 100, LCP < 2.5 s, CLS < 0.1.

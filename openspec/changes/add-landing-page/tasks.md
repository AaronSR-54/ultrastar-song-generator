# Tasks

## 1. Andamiaje del sitio

- [x] 1.1 Crear `site/` con Next.js 16 (App Router), React 19, TypeScript estricto y Tailwind CSS 4; fijar versiones con `package-lock.json` y dejar el proyecto fuera del workspace Cargo. Verificar con `cd site && npm ci && npm run build`.
- [x] 1.2 Añadir scripts `dev`, `build`, `start`, `lint` (ESLint 9 + `eslint-config-next`), `typecheck` (`tsc --noEmit`) y `test` (Vitest), más `next.config.ts`, `tsconfig.json`, `postcss.config.mjs` y `eslint.config.mjs`. Verificar que `npm run lint`, `npm run typecheck` y `npm run test` terminan sin errores.
- [x] 1.3 Ampliar `.gitignore` con `site/node_modules/`, `site/.next/`, `site/.vercel/`, `site/out/` y `site/coverage/`, y comprobar con `git status` que no aparece ningún artefacto generado.
- [x] 1.4 Escribir `site/README.md` con requisitos, comandos, estructura, despliegue en Vercel (`site/` como root, Node 22) y el contrato de nombres de asset. Verificar que los comandos documentados se ejecutan tal cual.

## 2. Sistema de diseño y componentes base

- [x] 2.1 Crear `src/styles/tokens.css` con la paleta, radios, espaciado y escala tipográfica de `crates/desktop/src/theme/`, y mapearla en Tailwind con `@theme inline`. Verificar con `tests/tokens.test.ts`, que parsea `palette.rs` y falla si un color o `RADIUS` no coincide con los tokens CSS.
- [x] 2.2 Implementar las primitivas `ui/button.tsx`, `ui/mono-label.tsx`, `ui/section.tsx` y `ui/icon.tsx` (SVG inline de 24 × 24, trazo 1.6–2, `currentColor`), con hover de 140 ms, pulsación de 70 ms, foco de 1 px y sin radios. Verificar que `npm run build` compila y que el HTML generado no contiene clases de `border-radius` en botones ni tarjetas.
- [x] 2.3 Montar `site-header.tsx` (sticky, hairline inferior, anclas y CTA compacta) y `site-footer.tsx` (licencia, enlaces, micro-línea). Verificar la navegación por anclas con teclado y que el foco se ve en todos los enlaces.
- [x] 2.4 Definir la retícula de fondo (puntos de 32 px al 7 %), las hairlines y la sombra única de elevación como utilidades reutilizables. Verificar visualmente en 360 px y 1440 px que la retícula no produce bandas ni moiré.

## 3. Contenido y secciones

- [x] 3.1 Construir el hero con eyebrow mono, `h1`, subtítulo, CTA, enlace secundario, línea de confianza y la ventana de app reconstruida en HTML/CSS. Verificar con capturas en 360/768/1024/1440 px que el hero no tiene texto huérfano ni solapamientos.
- [x] 3.2 Implementar la sección «Cómo funciona» con las seis etapas del pipeline, números mono y conectores de 1 px. Verificar que los nombres y descripciones coinciden con la tabla de etapas del `README.md`.
- [x] 3.3 Implementar «Todo ocurre en tu equipo» con el diagrama YouTube → equipo → carpeta y los tres puntos de privacidad. Verificar que la copia coincide con la sección de privacidad del README y no afirma que todo el tráfico sea local.
- [x] 3.4 Implementar «La letra, medida» con la franja de letra y la lane de notas en SVG (grid, notas, nota activa) y el texto sobre pYIN/fonemas. Verificar que el SVG es `aria-hidden` con descripción textual y que se ve nítido a 2× de densidad.
- [x] 3.5 Implementar «Lista para UltraStar» con el árbol de exportación en panel inset, fragmento `.txt` y detección de `SongDir1`. Verificar que el árbol y las etiquetas coinciden con la sección de exportación del README.
- [x] 3.6 Implementar «Requisitos» con las tres columnas por plataforma, herramientas en el `PATH`, modelos del primer uso y avisos de SmartScreen/Gatekeeper/permisos. Verificar que cada requisito aparece en el README y que los comandos son copiables.
- [x] 3.7 Implementar la FAQ con `<details>` nativos, icono `+`/`−` y seis entradas (sin binario para mi Mac, avisos del sistema, herramientas que faltan, modelos, idioma, formato). Verificar que se abre con teclado y que el contenido no repite secciones anteriores palabra por palabra.

## 4. Descargas y detección de plataforma

- [x] 4.1 Implementar `src/lib/platforms.ts` con `detectPlatform()` puro (Client Hints, user agent, iPadOS táctil, Windows/Linux ARM, macOS Intel) y el mapa a assets. Verificar con `tests/platforms.test.ts`, tabla de casos que cubra todas las ramas y `npm run test` en verde.
- [x] 4.2 Implementar `src/lib/downloads.ts` en servidor con la consulta ISR a la API de GitHub (300 s, token opcional), los nombres de asset estables y los estados `available`/`missing-assets`/`unavailable`. Verificar con `tests/assets.test.ts` que cada asset de la landing aparece literalmente en `.github/workflows/release.yml`.
- [x] 4.3 Construir `download-panel.tsx` y `platform-card.tsx`: tres tarjetas en el HTML inicial, CTA neutral, detección al montar, chip «Tu sistema», selector manual con radios nativos y `aria-live`. Verificar que el HTML de `curl` (sin JS) contiene los tres enlaces y que montar el componente no mueve el layout (CLS 0 en medición local).
- [x] 4.4 Añadir el enlace a `SHA256SUMS.txt`, el comando de verificación por plataforma y las notas de instalación de cada tarjeta. Verificar que el comando copiado funciona sobre un ZIP descargado de la release.
- [x] 4.5 Implementar los estados de release ausente o API caída (enlaces a releases y a compilar, sin CTA rota) y probarlos con datos simulados en los tests de `downloads.ts`. Verificar que ninguna tarjeta enlaza a un asset cuando `missing-assets` o `unavailable` está activo.

## 5. Metadatos, accesibilidad y calidad visual

- [x] 5.1 Configurar `metadata` (título, descripción, canónica, Open Graph, Twitter), `lang="es"`, JSON-LD `SoftwareApplication` y `app/icon.svg`. Verificar que el HTML servido contiene todas las etiquetas y que no hay dos `h1`.
- [x] 5.2 Crear `scripts/og.mjs` con `sharp` y una plantilla SVG que replique la retícula, el logo y el titular, y versionar `public/og.png` a 1200 × 630. Verificar dimensiones y que la URL de `og:image` responde 200 en el build local.
- [x] 5.3 Añadir skip link, landmarks, foco visible, `prefers-reduced-motion`, contraste AA y `aria-live` en el selector. Verificar con navegación solo de teclado y con `npx @axe-core/cli http://localhost:3000` sin violaciones críticas.
- [x] 5.4 Ejecutar Lighthouse móvil sobre `npm run build && npm start` y ajustar hasta Rendimiento ≥ 95 y Accesibilidad/Buenas prácticas/SEO = 100, con LCP < 2.5 s y CLS < 0.1.
- [x] 5.5 Revisar la dirección de arte con capturas reales en 360/768/1024/1440 px contra la lista del `design.md` (composición asimétrica, un solo acento, sin radios, sin degradados, sin glassmorphism, hairlines alineadas) y corregir hasta que no queden desalineaciones ni secciones planas. Verificar que cada captura pasa la checklist y adjuntarlas a la revisión.

## 6. CI del sitio

- [x] 6.1 Añadir `.github/workflows/site.yml` con Node 22, cache npm, `npm ci`, lint, typecheck, test y build, filtrado por `paths` de `site/**`. Verificar con `actionlint` o `npx yaml-lint` que el YAML es válido y que un cambio solo en `crates/` no dispara el workflow.
- [ ] 6.2 Comprobar en un PR de prueba que el workflow falla al introducir un error de tipos en `site/` y vuelve a verde al corregirlo.

## 7. Release automática con assets

- [x] 7.1 Añadir al `release.yml` el job `publish` (`needs: build`, solo tags `v*`, `contents: write`) que descargue los artefactos, normalice los ZIP con `zip -j`, aplique `chmod +x` a los binarios Unix, genere `SHA256SUMS.txt` y publique con `softprops/action-gh-release@v2`. Verificar con `actionlint` que la sintaxis y los permisos son válidos.
- [x] 7.2 Verificar el empaquetado en local: descomprimir los tres ZIP, comprobar que los binarios Unix conservan el bit de ejecución y que `sha256sum -c SHA256SUMS.txt` valida los tres archivos.
- [ ] 7.3 Ejecutar `workflow_dispatch` y comprobar que genera artefactos sin publicar release; después, tras el merge, empujar `v0.1.0` y verificar que existen la release, los cuatro assets y las URLs `releases/latest/download/<asset>`.
- [x] 7.4 Actualizar `.github/workflows/release.yml` y `site/README.md` con el contrato de nombres y el procedimiento de tag; verificar que `tests/assets.test.ts` sigue verde.

## 8. Despliegue y verificación final

- [ ] 8.1 Importar el repositorio en Vercel con `site/` como root, Node 22, `npm ci` y sin variables obligatorias; documentar el `GITHUB_TOKEN` opcional. Verificar que el despliegue de producción responde 200 en `/`.
- [x] 8.2 Comprobar la detección en Windows, macOS Apple Silicon y Linux (real o emulación de user agent): la CTA coincide con el asset y el selector manual permite cambiar. Verificar también iOS/Android y una arquitectura sin binario.
- [ ] 8.3 Añadir al `README.md` una sección de descarga que enlace a la landing y al archivo de sumas, y verificar que los enlaces resuelven en producción.
- [x] 8.4 Ejecutar la verificación completa: `cd site && npm ci && npm run lint && npm run typecheck && npm run test && npm run build`, `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` y `openspec validate add-landing-page --strict`. Verificar que todo termina en verde.

## Workflow follow-up

- Archivar el cambio con `openspec-archive-change` cuando la revisión y el despliegue estén cerrados.
- Confirmar que la spec archivada de `landing-page` y `app-distribution` pasa a `openspec/specs/` sin placeholders.

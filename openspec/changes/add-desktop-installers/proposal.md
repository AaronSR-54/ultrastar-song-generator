# Proposal

## Why

Hoy la única forma de obtener la app es compilarla con Cargo o descargar un binario suelto del workflow de release. Eso deja fuera a cualquier usuario que no tenga Rust: el binario no se integra en el menú de aplicaciones, no registra un desinstalador y en Windows/macOS no tiene icono ni metadatos del sistema. La app ya está completa y el pipeline se usa de principio a fin, así que la distribución es ahora el paso que impide que alguien la instale como cualquier programa de escritorio.

## What Changes

- El workflow de release pasa a producir un instalador por sistema operativo, además del binario actual:
  - **Windows:** `.msi` que instala por usuario (sin UAC), crea acceso directo en el menú Inicio, registra entrada de desinstalación en *Aplicaciones instaladas* y no abre consola.
  - **macOS:** `.dmg` con `UltraStar Song Generator.app` (bundle id, versión, icono y `Info.plist` mínimos) y enlace a *Applications* para arrastrar; arquitectura arm64 (la matriz actual).
  - **Linux:** `.deb` que instala el binario en `/usr/lib/usdx`, un lanzador en `/usr/bin`, entrada `.desktop` con categorías y los iconos en los tamaños estándar hicolor.
- La versión de los tres artefactos se toma de `workspace.package.version`; no se duplica en ningún manifiesto de empaquetado.
- Los instaladores se adjuntan al GitHub Release del tag y también se generan con `workflow_dispatch` como artefactos de la ejecución.
- Se añade un icono de aplicación de origen (PNG de alta resolución) del que se derivan `.ico`, `.icns` y los PNG del tema hicolor; también se usa como icono de la ventana.
- Se documenta qué herramientas del sistema siguen siendo responsabilidad del usuario (`yt-dlp`, `ffmpeg`/`ffprobe`, Node o Deno, `espeak-ng`) y la app mantiene el aviso existente en **Ajustes** cuando falta alguna.
- **BREAKING (solo usuarios de los binarios sueltos):** el release deja de publicar únicamente el ejecutable desnudo; el `.tar.gz`/`.zip` por plataforma pasa a ser un artefacto secundario, no la vía de instalación recomendada.
- **Fuera de alcance:** firma de código y notarización (Windows Authenticode, Apple Developer ID), empaquetar las herramientas externas o los pesos de los modelos, autoactualización, tiendas (Microsoft Store, Mac App Store, Flathub) y builds para Windows arm64 / macOS x86_64 / Linux arm64.

## Capabilities

### New Capabilities

- `desktop-distribution`: generación, contenido y verificación de los instaladores nativos por sistema operativo, incluidos metadatos, integración con el escritorio, versión y publicación en el release.

### Modified Capabilities

Ninguna. La detección de herramientas externas y el aviso en Ajustes ya existen y no cambian de comportamiento.

## Impact

- **CI/CD:** `.github/workflows/release.yml` gana pasos por sistema operativo (WiX en Windows, `hdiutil` en macOS, `dpkg-deb` en Linux) y publica los instaladores; `ci.yml` no cambia salvo que se añada una comprobación de que el empaquetado no rompe el workspace.
- **Código:** `crates/desktop/src/lib.rs` fija el icono de la ventana; se añaden activos en `assets/` (icono fuente y derivados). No cambia la lógica de la app ni del pipeline.
- **Empaquetado:** ficheros nuevos de configuración por plataforma (`wix`, `Info.plist`, plantilla `control` de Debian, `.desktop`) y un script/objetivo único para invocarlos en local.
- **Documentación:** nueva guía de instalación para usuarios finales y actualización de la sección 1 del `README.md` para que «Compilar» deje de ser el camino principal.
- **Licencias:** se mantiene la regla de `docs/licenses.md`: el instalador no incluye `ffmpeg` GPL ni pesos de Demucs/Whisper.
- **Riesgo:** bajo en código, medio en release (WiX y `dpkg-deb` son sensibles a rutas, permisos y arquitecturas). La mitigación es ejecutar el empaquetado en el mismo job que ya hace el smoke test `--selftest` y validar cada artefacto antes de publicarlo.

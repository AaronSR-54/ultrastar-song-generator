# Design

## Context

- El release actual (`.github/workflows/release.yml`) compila `usdx-desktop` en Windows, macOS y Linux, ejecuta `--selftest` y sube el binario desnudo como artefacto. No hay publicación en GitHub Releases ni metadatos de sistema.
- El binario es una app `eframe`/`egui` 0.37. En Linux solo enlaza dinámicamente `libasound`, `libstdc++`, `libgcc_s`, `libm` y `libc`; ONNX Runtime se enlaza estáticamente. `winit` carga X11/Wayland por `dlopen`, así que esas librerías no aparecen en `ldd`.
- En Windows el target MSVC usa CRT dinámico: el ejecutable depende de `VCRUNTIME140.dll` / `VCRUNTIME140_1.dll`, que no forman parte de una instalación limpia de Windows.
- No existen activos de icono en el repositorio; los iconos de la UI son vectoriales y se pintan con `egui`.
- La versión vive en `[workspace.package] version` (`Cargo.toml`) y `usdx-desktop` la hereda con `version.workspace = true`.
- La app ya detecta `yt-dlp`, `ffmpeg`, `ffprobe`, Node/Deno y `espeak-ng` (`Toolchain::detect`) y avisa en **Ajustes**.
- Ver `proposal.md` para la motivación y `specs/desktop-distribution/spec.md` para el contrato de comportamiento.

## Goals / Non-Goals

**Goals:**

- Un camino de empaquetado reproducible y auditable desde el repositorio: `packaging/` contiene todo lo específico de plataforma y el workflow solo lo invoca.
- Que el resultado instalado pase el mismo `--selftest` que ya usa el release.
- Que la versión y el nombre del producto tengan una sola fuente (`Cargo.toml`).

**Non-Goals:**

- Firma Authenticode, notarización, stapling o hardened runtime con entitlements.
- Autoactualización, repositorios APT propios, winget/Homebrew/Flathub.
- Múltiples arquitecturas por SO (se mantiene la matriz actual: Windows x86_64, macOS arm64, Linux amd64).
- Empaquetar herramientas externas o pesos de modelos.

## Decisions

### 1. Scripts nativos por plataforma en `packaging/`, sin un empaquetador único

Cada SO usa la herramienta nativa que ya está en su runner: WiX para MSI, `hdiutil`/`iconutil` para DMG y `dpkg-deb` para DEB. Un script por plataforma (`packaging/windows/package.ps1`, `packaging/macos/package.sh`, `packaging/linux/package.sh`) recibe `VERSION` y la ruta del binario, y deja el artefacto en `dist/`.

**Por qué no `cargo-packager`:** es la opción "un comando para todo", produce DMG y DEB correctamente, pero su formato WiX figura como no soportado en su propia documentación de integración y su `WixConfig` no expone el *scope* de instalación, que en Tauri deriva en `perMachine`. Nuestro requisito es instalación por usuario sin UAC, así que el MSI necesita autoría propia. Mantener un solo empaquetador para dos de tres formatos añade una dependencia en *public preview* sin ahorrar el trabajo real.

**Alternativas descartadas:**

- `cargo-wix`: genera un `.wxs` a partir de metadatos, pero su plantilla está pensada para instalación *per-machine* y personalizarla acaba siendo el mismo `.wxs` escrito a mano, con una dependencia extra.
- `cargo-dist`: no produce DMG ni DEB; solo MSI dentro de su conjunto de instaladores.
- Scripts solo para Linux/macOS y NSIS en Windows: NSIS instala por usuario fácilmente, pero el usuario pidió MSI y MSI es lo que esperan entornos gestionados.

### 2. Windows: MSI con WiX v5, `Scope="perUser"`

- WiX se instala en el job con `dotnet tool install --global wix --version 5.*` (los runners de Windows traen .NET). Se fija la versión mayor para que la sintaxis no cambie por sorpresa.
- `packaging/windows/UltraStarSongGenerator.wxs` declara `<Package Scope="perUser" ...>` (WiX 4+), con:
  - `Directory` bajo `LocalAppDataFolder` → `%LOCALAPPDATA%\Programs\UltraStar Song Generator`.
  - Componente del ejecutable con `KeyPath` de archivo y acceso directo en `ProgramMenuFolder` (menú Inicio del usuario) con `Icon` y `WorkingDirectory`.
  - `MajorUpgrade` con `UpgradeCode` fijo (GUID generado una vez y comprometido) para que una versión nueva sustituya a la anterior sin dejar duplicados.
  - Entrada en *Aplicaciones instaladas* automática a partir de `Package Name/Version/Manufacturer` y `Icon`.
- El binario ya se compila con `windows_subsystem = "windows"`, así que el MSI no necesita nada para ocultar la consola.
- `ProductVersion` de MSI admite solo `major.minor.build` (sin prerelease). El script normaliza `0.2.0-beta.1` → `0.2.0` y deja la etiqueta completa en el nombre del archivo y en `DisplayVersion` mediante un `Property`/`RegistryValue` propio si hace falta.
- **LaunchCondition de CRT:** antes de instalar se comprueba que existen `VCRUNTIME140.dll` y `VCRUNTIME140_1.dll` en `System32`; si faltan, el MSI muestra un mensaje que enlaza a *Microsoft Visual C++ Redistributable 2015-2022 (x64)* y aborta. Se descarta `+crt-static` porque las librerías estáticas precompiladas de ONNX Runtime se compilan con `/MD` y mezclar CRTs es una fuente conocida de fallos de enlace y de runtime; el redistribuible es la solución soportada.
- Los avisos de ICE por componentes por usuario (`ICE38`, `ICE64`) se silencian explícitamente (`-sice:ICE38 -sice:ICE64`) tras comprobar que cada componente usa una clave HKCU o un archivo como `KeyPath`.

### 3. macOS: `.app` generado por script + DMG con `hdiutil`

- `packaging/macos/package.sh` construye `dist/UltraStar Song Generator.app/Contents/{MacOS,Resources}`:
  - `Info.plist` con `CFBundleIdentifier=com.aaronsr.ultrastar-song-generator`, `CFBundleName`, `CFBundleDisplayName`, `CFBundleShortVersionString` y `CFBundleVersion` (desde `VERSION`, normalizada a tres componentes), `CFBundlePackageType=APPL`, `CFBundleExecutable=usdx-desktop`, `CFBundleIconFile=AppIcon`, `LSMinimumSystemVersion=11.0`, `NSHighResolutionCapable=true` y `LSApplicationCategoryType=public.app-category.music`.
  - Binario copiado a `Contents/MacOS/usdx-desktop` con permisos `755`.
  - `AppIcon.icns` generado en el propio job con `iconutil` a partir del iconset comprometido (`assets/icons/AppIcon.iconset/*.png`), evitando depender de herramientas de conversión de terceros.
- El DMG se crea con `hdiutil create -format UDZO -fs HFS+ -volname "UltraStar Song Generator"` sobre una carpeta de staging que contiene el `.app` y un symlink `Applications -> /Applications`. Sin fondo ni posiciones personalizadas: la prioridad es que funcione y sea reproducible.
- No se firma. `codesign` no se ejecuta. El job verifica el binario antes de montar el DMG, y después monta el DMG y ejecuta `Contents/MacOS/usdx-desktop --selftest`.
- El binario solo es válido en arm64 (runner `macos-latest` es Apple Silicon). Se declara `LSArchitecturePriority` no es necesario; el nombre del artefacto lleva `arm64`.

### 4. Linux: `.deb` construido con `dpkg-deb`

- Árbol de staging:
  - `/usr/lib/ultrastar-song-generator/usdx-desktop` (binario `755`).
  - `/usr/bin/usdx-desktop` → symlink al binario.
  - `/usr/share/applications/ultrastar-song-generator.desktop` con `Name=UltraStar Song Generator`, `Exec=usdx-desktop`, `Icon=ultrastar-song-generator`, `Terminal=false`, `Categories=AudioVideo;Audio;Utility;`, `StartupWMClass=usdx-desktop`.
  - `/usr/share/icons/hicolor/{16,24,32,48,64,128,256,512}x{...}/apps/ultrastar-song-generator.png`.
  - `/usr/share/doc/ultrastar-song-generator/{copyright,changelog.gz}` a partir de `LICENSE`/`README`.
- `control` generado desde un template: `Package: ultrastar-song-generator`, `Version` desde `VERSION`, `Architecture: amd64`, `Maintainer` con el autor del proyecto, `Section: sound`, `Priority: optional`, `Homepage`, `Description`.
- `Depends`: `${shlibs:Depends}` generado con `dpkg-shlibdeps` sobre el binario (captura `libasound2`, `libstdc++6`, `libgcc-s1`, `libc6`) **más** la lista de librerías que `winit`/`rfd` cargan con `dlopen`: `libxkbcommon0`, `libxkbcommon-x11-0`, `libwayland-client0`, `libx11-6`, `libxcb1`, `libxcursor1`, `libxrandr2`, `libxi6`. Esta lista se contrasta en el job de verificación ejecutando el binario en un contenedor limpio con solo `Depends` instalados.
- `Recommends: ffmpeg, yt-dlp, espeak-ng, nodejs` para que `apt install ./archivo.deb` ofrezca las herramientas que la app necesita, sin convertirlas en dependencias duras (el usuario puede usar la app sin ellas y ver el aviso en Ajustes).
- No hay `postinst`/`postrm`: `dpkg` dispara `update-desktop-database` e `icon-cache` mediante los triggers estándar de `desktop-file-utils` y `hicolor-icon-theme`, que se declaran en `Depends`.
- La desinstalación no toca `~/.local/share/usdx` ni `~/.config` (la app usa `directories`).
- Base: `ubuntu-latest` (24.04 al escribir esto). El `.deb` va dirigido a esa versión y posteriores; se documenta.

### 5. Iconos con fuente única comprometida

- `assets/icon.png` (1024×1024, fondo y glifo de nota musical acordes al tema) es el original.
- Derivados comprometidos: `assets/icon.ico` (16, 24, 32, 48, 64, 128, 256), `assets/icon.icns` no se compromete; se generan `assets/icons/AppIcon.iconset/icon_*.png` para macOS y `assets/icons/hicolor/<size>/ultrastar-song-generator.png` para Linux.
- `crates/desktop/src/lib.rs` fija el icono de la ventana con `eframe::icon_data::from_png_bytes(include_bytes!("../../assets/icon.png"))` (ya existe en eframe 0.37). Si la decodificación falla en tiempo de compilación/ejecución se omite el icono en vez de abortar; el `--selftest` no depende de él.
- Los binarios se generan una sola vez y se comprometen para que el empaquetado no necesite ImageMagick/PIL en CI.

### 6. Versión: `cargo metadata` como única fuente

- Un paso previo de cada job calcula `VERSION=$(cargo metadata --no-deps --format-version 1 | jq -r '.packages[] | select(.name=="usdx-desktop") | .version')` y la exporta.
- `packaging/version.sh` (y su equivalente PowerShell) centraliza la normalización: `display` (cadena completa) y `numeric` (tres componentes numéricos para MSI/plist).
- El nombre de los artefactos sigue un patrón fijo: `UltraStar-Song-Generator-<display>-windows-x64.msi`, `UltraStar-Song-Generator-<display>-macos-arm64.dmg`, `ultrastar-song-generator_<display>_amd64.deb`.
- La verificación compara la versión declarada en cada artefacto (`7z`/`msiextract`, `PlistBuddy`, `dpkg-deb -f`) con `VERSION` y falla si no coincide.

### 7. Publicación: artefactos en cada run, release solo en tags

- Los tres jobs siguen subiendo el artefacto con `actions/upload-artifact`, ahora el instalador.
- Un job `publish` (necesita `contents: write` y `needs: build`) descarga los tres artefactos y:
  - En tags `v*`: crea o reutiliza el release con `gh release create "$GITHUB_REF_NAME" --generate-notes --verify-tag` y sube los archivos con `gh release upload --clobber`. Adjunta además la guía de instalación (`docs/install.md`) como nota del release.
  - En `workflow_dispatch`: no crea release; los artefactos de la ejecución son la entrega.
- Se añade un `SHA256SUMS.txt` generado en `publish` para que los usuarios verifiquen la descarga.

### 8. Verificación real de cada instalador

Cada job instala el artefacto que acaba de producir y ejecuta el binario instalado:

- **Windows:** `msiexec /i dist\*.msi /qn /l*v install.log`, comprobar que existe `%LOCALAPPDATA%\Programs\UltraStar Song Generator\usdx-desktop.exe`, ejecutarlo con `--selftest`, y `msiexec /x` sobre el mismo `.msi` para comprobar la desinstalación. El directorio de datos (`%LOCALAPPDATA%\usdx`) no se toca.
- **macOS:** montar el DMG, ejecutar `.../Contents/MacOS/usdx-desktop --selftest`, comprobar `Info.plist` con `plutil -lint` y `PlistBuddy` (bundle id, versión, icono) y desmontar.
- **Linux:** `sudo apt-get install -y ./dist/*.deb`, `usdx-desktop --selftest`, comprobar `desktop-file-validate`, `dpkg -L` para las rutas esperadas, y `sudo apt-get purge -y ultrastar-song-generator`.
- Cualquier fallo detiene el job antes de `publish`.

## Risks / Trade-offs

- **Rutas y permisos en WiX** → mantener el `.wxs` mínimo, ejecutar el MSI en el runner y revisar `install.log`; fijar la versión mayor de WiX.
- **`MajorUpgrade` mal configurado deja dos versiones instaladas** → `UpgradeCode` fijo comprometido y prueba de actualización (instalar versión anterior + nueva) en una tarea de verificación manual antes del primer release real.
- **`dpkg-shlibdeps` depende de la distro del runner** → el `.deb` se declara para `ubuntu-latest` y posteriores; la lista de `dlopen` se valida en un contenedor limpio.
- **Dependencias `dlopen` incompletas en Linux** (Wayland vs X11, cursores) → la verificación arranca la app con `--selftest` en un contenedor con solo `Depends`; si el arranque real falla, se prueba con `xvfb-run` y se añade la librería que falte.
- **Instaladores sin firmar**: SmartScreen y Gatekeeper asustan al usuario → el release incluye instrucciones explícitas y checksums; la firma queda como cambio futuro.
- **DMG arm64 únicamente** deja fuera Macs Intel → documentado; añadir x86_64 es una entrada más en la matriz y no cambia los scripts.
- **Release con permisos de escritura** en un workflow que hoy es de solo lectura → el `permissions: contents: write` se limita al job `publish`, no al workflow completo.
- **CRT dinámico en Windows** deja fuera equipos sin redistribuible → `LaunchCondition` con mensaje accionable en vez de un fallo críptico al arrancar.

## Migration Plan

1. Aterrizar scripts y activos; ejecutar el empaquetado en local (Linux) para validar el `.deb` y el formato de los scripts.
2. Ejecutar `workflow_dispatch` y descargar los tres artefactos; instalar cada uno en una máquina virtual limpia (Windows 11 sin VS, macOS 14+, Ubuntu 24.04) y comprobar instalación, arranque, `--selftest` y desinstalación.
3. Publicar un tag de prueba (`v0.1.1`) y comprobar el release y los checksums.
4. Actualizar `README.md` y añadir `docs/install.md`.

Rollback: los instaladores no alteran nada fuera de sus rutas de instalación y dejan intactos los datos de usuario; si un artefacto sale mal, se borra del release y se mantiene el binario anterior. El workflow antiguo se puede restaurar revirtiendo el commit de `.github/workflows/release.yml` sin tocar el código de la app.

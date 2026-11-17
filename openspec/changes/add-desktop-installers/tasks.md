# Tasks

## 1. Activos de icono

- [x] 1.1 Crear `assets/icon.png` (1024×1024, nota musical sobre fondo sólido acorde al tema). Verificación: `file assets/icon.png` informa PNG de 1024×1024 y la imagen se ve nítida al abrirla.
- [x] 1.2 Derivar y comprometer `assets/icon.ico` (16, 24, 32, 48, 64, 128, 256), `assets/icons/AppIcon.iconset/icon_*.png` y `assets/icons/hicolor/<size>x<size>/apps/ultrastar-song-generator.png` para los tamaños 16, 24, 32, 48, 64, 128, 256 y 512. Verificación: `identify assets/icon.ico` lista los siete tamaños y `find assets/icons -name '*.png' | wc -l` cuenta 8 iconset + 8 hicolor.
- [x] 1.3 Fijar el icono de la ventana en `crates/desktop/src/lib.rs` con `eframe::icon_data::from_png_bytes(include_bytes!("../../assets/icon.png"))`, ignorando un error de decodificación sin abortar. Verificación: `cargo build -p usdx-desktop` compila y la app abierta en Linux muestra el icono en la barra de tareas.

## 2. Base de empaquetado y versión

- [x] 2.1 Añadir `packaging/version.sh` y `packaging/version.ps1`, que leen la versión con `cargo metadata --no-deps` para `usdx-desktop` y emiten `VERSION_DISPLAY` y `VERSION_NUMERIC` (tres componentes, sin prerelease). Verificación: con la versión actual ambos imprimen `VERSION_DISPLAY=0.1.0` y `VERSION_NUMERIC=0.1.0`; probando `0.2.0-beta.1` en un fixture, `VERSION_NUMERIC=0.2.0`.
- [x] 2.2 Añadir `dist/` a `.gitignore` y crear `packaging/README.md` que explique los tres scripts y sus requisitos. Verificación: `git status --porcelain` no muestra `dist/` tras generar un artefacto y el README menciona WiX, `hdiutil` y `dpkg-deb`.

## 3. Instalador de Linux (.deb)

- [x] 3.1 Escribir `packaging/linux/package.sh` que monte el árbol de staging, calcule `Depends` con `dpkg-shlibdeps` más las librerías `dlopen` de `winit`/`rfd`, rellene `control` desde una plantilla y genere `dist/ultrastar-song-generator_<versión>_amd64.deb` con `dpkg-deb --build --root-owner-group`. Verificación: el script termina con código 0 y `dpkg-deb -I dist/*.deb` muestra `Package`, `Version`, `Architecture: amd64`, `Depends` con `libxkbcommon0` y `Recommends` con `ffmpeg` y `yt-dlp`.
- [x] 3.2 Añadir `packaging/linux/ultrastar-song-generator.desktop` y la plantilla de `control`, con nombre visible correcto, `Exec=usdx-desktop`, `Icon=ultrastar-song-generator`, `Terminal=false` y `Categories=AudioVideo;Audio;Utility;`. Verificación: `desktop-file-validate packaging/linux/ultrastar-song-generator.desktop` no emite errores y `dpkg-deb -c dist/*.deb` incluye el `.desktop` y los PNG de hicolor.
- [ ] 3.3 Verificar el paquete en un contenedor limpio `ubuntu:24.04`: instalar con `apt-get install ./dist/*.deb`, ejecutar `usdx-desktop --selftest` y comprobar `selftest ok`, crear un proyecto en `~/.local/share/usdx`, purgar el paquete y confirmar que el proyecto sigue existiendo. Verificación: la secuencia completa pasa dentro del contenedor y `dpkg -L` ya no lista el ejecutable tras purgar.
- [ ] 3.4 Añadir al job de Linux de `release.yml` el paso de empaquetado y la verificación de instalación (`apt-get install`, `--selftest`, `apt-get purge`). Verificación: una ejecución `workflow_dispatch` produce el `.deb` y el job falla si `--selftest` no imprime `selftest ok`.

## 4. Instalador de macOS (.dmg)

- [ ] 4.1 Crear `packaging/macos/Info.plist` con bundle id `com.aaronsr.ultrastar-song-generator`, nombre visible, versión, ejecutable e icono, y `packaging/macos/package.sh` que monte el `.app`, genere `AppIcon.icns` con `iconutil` desde `assets/icons/AppIcon.iconset` y cree el DMG UDZO con enlace a *Applications*. Verificación: `bash -n packaging/macos/package.sh` pasa y `plutil -lint packaging/macos/Info.plist` devuelve `OK` en el job de macOS.
- [ ] 4.2 Añadir al job de macOS de `release.yml` el empaquetado y la verificación: montar el DMG, comprobar con `PlistBuddy` bundle id y versión, ejecutar `Contents/MacOS/usdx-desktop --selftest` y desmontar. Verificación: una ejecución `workflow_dispatch` produce `UltraStar-Song-Generator-<versión>-macos-arm64.dmg` y el job falla si `--selftest` no imprime `selftest ok`.

## 5. Instalador de Windows (.msi)

- [ ] 5.1 Escribir `packaging/windows/UltraStarSongGenerator.wxs` con `Scope="perUser"`, instalación en `%LOCALAPPDATA%\Programs\UltraStar Song Generator`, acceso directo en `ProgramMenuFolder`, icono ARP, `MajorUpgrade` con `UpgradeCode` fijo y `LaunchCondition` de `VCRUNTIME140.dll`/`VCRUNTIME140_1.dll`. Verificación: `wix build` compila el `.wxs` en el runner de Windows sin errores.
- [ ] 5.2 Escribir `packaging/windows/package.ps1` que normalice la versión a tres componentes, invoque `wix build` con `-d Version=<numérica>` y las supresiones `ICE38`/`ICE64`, y deje `dist\UltraStar-Song-Generator-<versión>-windows-x64.msi`. Verificación: el script termina con código 0 y `Get-Item dist\*.msi` muestra el tamaño esperado.
- [ ] 5.3 Añadir al job de Windows de `release.yml` el empaquetado y la verificación: `msiexec /i ... /qn /l*v`, comprobar que existe el `.exe` en `%LOCALAPPDATA%\Programs\UltraStar Song Generator`, ejecutarlo con `--selftest` y desinstalar con `msiexec /x`. Verificación: una ejecución `workflow_dispatch` produce el `.msi`, el log no contiene errores y la desinstalación deja el directorio de instalación vacío.

## 6. Publicación y coherencia de versiones

- [ ] 6.1 Añadir los pasos de verificación de versión por artefacto (`7z`/`msiextract`, `PlistBuddy`, `dpkg-deb -f`) que comparan con `VERSION_DISPLAY` y fallan si no coinciden. Verificación: alterando temporalmente la versión esperada en el paso, el job falla con un mensaje que muestra las dos versiones.
- [ ] 6.2 Añadir el job `publish` con `permissions: contents: write` limitado a ese job, que descarga los tres artefactos, genera `SHA256SUMS.txt`, crea o reutiliza el release en tags `v*` con `gh release create/upload` y solo publica artefactos de ejecución en `workflow_dispatch`. Verificación: un `workflow_dispatch` deja los tres instaladores y `SHA256SUMS.txt` como artefactos de la ejecución; el workflow no pide permisos de escritura fuera de `publish`.
- [ ] 6.3 Comprobar la coherencia de nombres y arquitecturas de los tres artefactos (`windows-x64`, `macos-arm64`, `amd64`) y que ninguno incluye `ffmpeg`, `yt-dlp`, Node/Deno ni pesos de modelos. Verificación: `dpkg-deb -c`, `7z l` y `hdiutil`/`find` sobre el contenido montado no encuentran esos binarios ni archivos `.onnx`/`.bin` de modelos.

## 7. Documentación

- [ ] 7.1 Escribir `docs/install.md` con la instalación por SO, los requisitos de herramientas externas que siguen siendo del usuario, el aviso de SmartScreen/Gatekeeper para artefactos sin firmar, la verificación con `SHA256SUMS.txt` y cómo desinstalar en cada plataforma. Verificación: los comandos copiados desde el documento funcionan en una máquina limpia de cada SO y todas las rutas enlazadas existen.
- [x] 7.2 Reorganizar la sección 1 de `README.md` para que el instalador sea el camino principal y mover «Compilar» a `docs/setup.md`, dejando claro que los modelos se descargan en el primer uso. Verificación: el README ya no mezcla compilación con instalación y cada enlace nuevo resuelve a un fichero existente (`grep -o '](docs/[^)]*)' README.md` y comprobar uno a uno).

## 8. Verificación de integración

- [ ] 8.1 Ejecutar `workflow_dispatch`, descargar los tres instaladores y probarlos en máquinas limpias (Windows 11 sin Visual Studio, macOS 14+, Ubuntu 24.04): instalación, arranque, `--selftest`, entrada en el menú de aplicaciones e instalación de una canción de prueba. Verificación: los tres instalan y arrancan, y los datos de usuario sobreviven a la desinstalación.
- [ ] 8.2 Validar la actualización: instalar la versión anterior y luego la nueva sobre ella en Windows y Linux. Verificación: queda una sola entrada en *Aplicaciones instaladas* / `dpkg -l` y los proyectos siguen abriéndose.
- [ ] 8.3 Publicar un tag de prueba `v0.1.1`, comprobar el release con los tres instaladores y `SHA256SUMS.txt`, y verificar los checksums descargados. Verificación: `sha256sum -c SHA256SUMS.txt` pasa sobre los archivos descargados del release.

## Workflow follow-up

- Archivar el cambio con `openspec-archive-change` cuando la verificación de integración esté completa.
- Abrir un cambio futuro para firma de código y notarización (Authenticode, Developer ID).

# Instalación

Los instaladores se publican en la página de *Releases* del repositorio:

<https://github.com/AaronSR-54/ultrastar-song-generator/releases>

| Sistema | Archivo | Requisitos |
|---|---|---|
| Windows 10/11 x64 | `UltraStar-Song-Generator-<versión>-windows-x64.msi` | Visual C++ 2015-2022 Redistributable (x64) |
| macOS 11+ (Apple Silicon) | `UltraStar-Song-Generator-<versión>-macos-arm64.dmg` | arm64 |
| Ubuntu 24.04 x86_64 | `ultrastar-song-generator_<versión>_amd64.deb` | Ubuntu 24.04 o posterior |

Los instaladores **no** incluyen `yt-dlp`, `ffmpeg`, `espeak-ng` ni Node/Deno, ni los
pesos de los modelos: la app los detecta y avisa en **Ajustes** cuando falta
alguno. La sección [Herramientas externas](#herramientas-externas) dice cómo
instalarlos y [Modelos](#modelos) explica cuándo se descargan.

Los artefactos no están firmados todavía (no hay certificado Authenticode ni
notarización de Apple). Por eso Windows y macOS muestran un aviso la primera
vez; más abajo está cómo continuar de forma consciente.

## Verificar la descarga

Cada release publica los checksums junto a los artefactos:

- `SHA256SUMS-installers.txt` cubre los tres instaladores nativos (el que
  necesitas aquí).
- `SHA256SUMS.txt` cubre los ZIP portables que ofrece la página web.

Descarga el fichero que corresponda a lo que hayas bajado y comprueba el
instalador.

```bash
# Linux
sha256sum -c SHA256SUMS-installers.txt --ignore-missing
```

```bash
# macOS
shasum -a 256 -c SHA256SUMS-installers.txt --ignore-missing
```

```powershell
# Windows
Get-FileHash .\UltraStar-Song-Generator-*-windows-x64.msi -Algorithm SHA256
Get-Content .\SHA256SUMS-installers.txt
```

Compara el hash a mano: el valor calculado debe coincidir con la línea del
`.msi`. Si no coincide, borra el archivo y vuelve a descargarlo.

## Windows

1. Abre el `.msi` con doble clic.
2. SmartScreen muestra *«Windows protegió su PC»* porque el archivo no está
   firmado. Pulsa **Más información** → **Ejecutar de todas formas**. El aviso
   solo aparece hasta que el binario tenga reputación.
3. Si falta el redistribuible de Visual C++, el instalador se detiene con un
   mensaje que enlaza a
   <https://aka.ms/vs/17/release/vc_redist.x64.exe>. Instálalo y vuelve a
   ejecutar el `.msi`.
4. La instalación es **por usuario**: no pide UAC ni permisos de administrador.
   Copia la app en
   `%LOCALAPPDATA%\Programs\UltraStar Song Generator` y crea la entrada
   *UltraStar Song Generator* en el menú Inicio.

También puedes instalar sin interfaz desde PowerShell:

```powershell
Start-Process msiexec.exe -ArgumentList '/i', '.\UltraStar-Song-Generator-<versión>-windows-x64.msi', '/qn' -Wait
```

Desinstalar: **Configuración → Aplicaciones → Aplicaciones instaladas →
UltraStar Song Generator → Desinstalar**, o

```powershell
Start-Process msiexec.exe -ArgumentList '/x', '.\UltraStar-Song-Generator-<versión>-windows-x64.msi', '/qn' -Wait
```

Los proyectos, ajustes y modelos viven en `%APPDATA%\ultrastar-song-generator` y
no se tocan al desinstalar.

## macOS

1. Abre el `.dmg` y arrastra **UltraStar Song Generator.app** a *Applications*.
2. Gatekeeper bloquea la app sin firmar: al abrirla aparece *«no se puede abrir
   porque es de un desarrollador no identificado»*. Abre el Finder, haz **clic
   derecho → Abrir** sobre la app y confirma **Abrir** en el diálogo. Solo hace
   falta la primera vez.
   - Si macOS la marca como dañada por la cuarentena del `.dmg`, ejecuta
     `xattr -dr com.apple.quarantine "/Applications/UltraStar Song Generator.app"`.
3. El bundle es solo arm64: en un Mac Intel no arranca.

Desinstalar: arrastra la app de *Applications* a la papelera. Los datos
(`~/Library/Application Support/ultrastar-song-generator`) se conservan.

## Linux (Debian/Ubuntu)

```bash
sudo apt install ./ultrastar-song-generator_<versión>_amd64.deb
```

`apt` resuelve las dependencias del sistema (ALSA, X11/Wayland, cursores) y
avisa de los *Recommends* opcionales. Con `dpkg -i` también funciona, pero
tendrás que instalar a mano lo que falte.

Tras instalar:

- el comando `usdx-desktop` queda en el `PATH`;
- aparece **UltraStar Song Generator** en el menú de aplicaciones, con los
  iconos hicolor instalados;
- no hay `postinst`: la caché de iconos y de lanzadores la actualizan los
  *triggers* estándar de `desktop-file-utils` y `hicolor-icon-theme`.

El paquete está construido para Ubuntu 24.04 y posteriores. En otras
distribuciones puede funcionar, pero las versiones de las dependencias no están
verificadas.

Desinstalar:

```bash
sudo apt purge ultrastar-song-generator
```

Los proyectos, ajustes y modelos viven en `~/.local/share/ultrastar-song-generator`
y `~/.config/ultrastar-song-generator`; la desinstalación no los toca.

## Herramientas externas

La app arranca sin ellas, pero **Descarga** no funciona sin `yt-dlp` (y un
runtime de JavaScript), y el análisis de audio necesita `ffmpeg`/`ffprobe`.

| Herramienta | Windows | macOS | Ubuntu |
|---|---|---|---|
| `yt-dlp` | `winget install yt-dlp` | `brew install yt-dlp` | `pipx install yt-dlp` o `apt install yt-dlp` |
| Node.js o Deno | `winget install OpenJS.NodeJS` | `brew install node` | `apt install nodejs` |
| `ffmpeg`/`ffprobe` | `winget install Gyan.FFmpeg` | `brew install ffmpeg` | `apt install ffmpeg` |
| `espeak-ng` (opcional) | instalador de <https://github.com/espeak-ng/espeak-ng/releases> | `brew install espeak-ng` | `apt install espeak-ng` |

Después de instalarlas, reinicia la app para que refresque el `PATH`. **Ajustes**
muestra la versión detectada de cada una y marca en rojo las que faltan.

`espeak-ng` solo hace falta para medir sílabas con fonemas; sin él, una letra
online se estima por líneas y una letra escrita a mano no se puede medir.

## Modelos

Los pesos no viajan dentro del instalador. El primer procesamiento descarga
HTDemucs (~302 MB) y el alineador fonético (~318 MB); Whisper (~547 MB) solo se
descarga si no hay letra online y eliges transcribir. Se guardan en:

| Sistema | Carpeta |
|---|---|
| Windows | `%APPDATA%\ultrastar-song-generator\models` |
| macOS | `~/Library/Application Support/ultrastar-song-generator/models` |
| Linux | `~/.local/share/ultrastar-song-generator/models` |

Las variables `USDX_*_MODEL` permiten apuntar a copias propias. Detalles en
[`models.md`](models.md) y avisos de licencia en [`licenses.md`](licenses.md).

Si prefieres descargarlos a mano, la app también los busca en
`USDX_MODELS_DIR`, en `usdx-models` dentro de la carpeta temporal y en
`models/` junto al ejecutable:

```bash
# Linux/macOS
mkdir -p ~/.local/share/ultrastar-song-generator/models
cd ~/.local/share/ultrastar-song-generator/models
curl -L -O https://huggingface.co/StemSplitio/htdemucs-onnx/resolve/main/htdemucs.onnx
curl -L -O https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-large-v3-turbo-q5_0.bin
curl -L -O https://huggingface.co/onnx-community/wav2vec2-lv-60-espeak-cv-ft-ONNX/resolve/main/onnx/model_int8.onnx
mv model_int8.onnx wav2vec2-espeak-ctc-int8.onnx
sha256sum htdemucs.onnx ggml-large-v3-turbo-q5_0.bin wav2vec2-espeak-ctc-int8.onnx
```

En macOS la primera ruta es
`~/Library/Application Support/ultrastar-song-generator/models`.

```powershell
# Windows PowerShell
$dir = "$env:APPDATA\ultrastar-song-generator\models"
New-Item -ItemType Directory -Force $dir | Out-Null
Invoke-WebRequest -Uri "https://huggingface.co/StemSplitio/htdemucs-onnx/resolve/main/htdemucs.onnx" -OutFile "$dir\htdemucs.onnx"
Invoke-WebRequest -Uri "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-large-v3-turbo-q5_0.bin" -OutFile "$dir\ggml-large-v3-turbo-q5_0.bin"
Invoke-WebRequest -Uri "https://huggingface.co/onnx-community/wav2vec2-lv-60-espeak-cv-ft-ONNX/resolve/main/onnx/model_int8.onnx" -OutFile "$dir\wav2vec2-espeak-ctc-int8.onnx"
Get-FileHash "$dir\*" -Algorithm SHA256
```

## Comprobar la instalación

```bash
usdx-desktop --selftest
```

En Windows:

```powershell
& "$env:LOCALAPPDATA\Programs\UltraStar Song Generator\usdx-desktop.exe" --selftest
```

Debe terminar con `selftest ok`. No abre ventana ni usa la red: detecta
herramientas, valida el catálogo de modelos y crea, guarda y reabre un proyecto
temporal.

## Datos que sobreviven a todo

Ningún instalador escribe en las carpetas de usuario de la app. Proyectos,
ajustes y modelos quedan fuera del directorio de instalación, así que una
actualización o una desinstalación no los modifica. Para borrarlos de verdad hay
que eliminar esas carpetas a mano.

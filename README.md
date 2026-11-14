# UltraStar Song Generator

Aplicación de escritorio que convierte una URL de YouTube en una carpeta de canción jugable en UltraStar Deluxe / USDX: letra silabificada, notas con pitch y duración, audio, carátula y un editor para revisar el resultado antes de exportar.

Todo el análisis corre en local (HTDemucs ONNX + whisper.cpp + pYIN). No se sube audio a ningún servicio y no se usa ninguna API de IA en la nube.

- **Plataformas:** Windows, macOS y Linux.
- **Formato:** `.txt` de UltraStar, una sola voz (sin duetos en el MVP).
- **Licencia:** MIT OR Apache-2.0. Detalles y dependencias en [`docs/licenses.md`](docs/licenses.md).

## 1. Instalación

### 1.1 Requisitos

| Componente | Versión | Para qué |
|---|---|---|
| Rust | 1.89 o superior | Compilar la app |
| `yt-dlp` | 2025.09 o superior | Descargar de YouTube |
| Node.js ≥ 18 o Deno | cualquiera reciente | `yt-dlp` lo necesita para resolver YouTube |
| `ffmpeg` y `ffprobe` | 6.0 o superior | Convertir a WAV y leer duración |
| Audio de salida | — | Reproducir en el editor (opcional) |

`yt-dlp`, `ffmpeg` y `ffprobe` deben estar en el `PATH`. La app los detecta al arrancar y muestra su versión en la barra de estado; si falta alguno, la etapa **Descarga** falla con un mensaje explícito antes de descargar nada.

### 1.2 Compilar

```bash
git clone https://github.com/AaronSR-54/ultrastar-song-generator.git
cd ultrastar-song-generator
cargo build --release -p usdx-desktop
```

El ejecutable queda en `target/release/usdx-desktop` (`.exe` en Windows).

### 1.3 Modelos

Los pesos no se incluyen en el repositorio ni en el binario. Descarga estos dos archivos y colócalos en una carpeta, por ejemplo `%TEMP%\usdx-models` o `~/.local/share/usdx/models`:

| Modelo | Archivo | Tamaño | SHA-256 |
|---|---|---|---|
| HTDemucs 4-stem | `htdemucs.onnx` | 316 446 953 B | `68d0bf16428ef66e692cdff8a9ccf28f1ef3f69440d57e58605a4cc55fcc5e74` |
| Whisper large-v3-turbo q5_0 | `ggml-large-v3-turbo-q5_0.bin` | 574 041 195 B | `394221709cd5ad1f40c46e6031ca61bce88931e6e088c188294c6d5a55ffa7e2` |

```bash
# Linux/macOS
mkdir -p ~/.local/share/usdx/models && cd ~/.local/share/usdx/models
curl -L -O https://huggingface.co/StemSplitio/htdemucs-onnx/resolve/main/htdemucs.onnx
curl -L -O https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-large-v3-turbo-q5_0.bin
sha256sum htdemucs.onnx ggml-large-v3-turbo-q5_0.bin
```

```powershell
# Windows PowerShell
New-Item -ItemType Directory -Force "$env:LOCALAPPDATA\usdx\models" | Out-Null
Invoke-WebRequest -Uri "https://huggingface.co/StemSplitio/htdemucs-onnx/resolve/main/htdemucs.onnx" -OutFile "$env:LOCALAPPDATA\usdx\models\htdemucs.onnx"
Invoke-WebRequest -Uri "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-large-v3-turbo-q5_0.bin" -OutFile "$env:LOCALAPPDATA\usdx\models\ggml-large-v3-turbo-q5_0.bin"
Get-FileHash "$env:LOCALAPPDATA\usdx\models\*" -Algorithm SHA256
```

Antes de arrancar, apunta la app a esos archivos:

```bash
export USDX_HTDEMUCS_MODEL="$HOME/.local/share/usdx/models/htdemucs.onnx"
export USDX_WHISPER_MODEL="$HOME/.local/share/usdx/models/ggml-large-v3-turbo-q5_0.bin"
```

En PowerShell:

```powershell
$env:USDX_HTDEMUCS_MODEL = "$env:LOCALAPPDATA\usdx\models\htdemucs.onnx"
$env:USDX_WHISPER_MODEL  = "$env:LOCALAPPDATA\usdx\models\ggml-large-v3-turbo-q5_0.bin"
```

También puedes editar las rutas en los campos **Modelo separación** y **Modelo Whisper** de la barra superior y pulsar **Aplicar modelos**.

> Los pesos de HTDemucs no están cubiertos por la licencia MIT del código de Demucs y se ofrecen para uso científico. Úsalos en local y no los redistribuyas. Si necesitas un producto comercial, sustitúyelos por pesos con licencia compatible.

### 1.4 Comprobar la instalación

```bash
target/release/usdx-desktop --selftest
```

Salida esperada (las versiones pueden variar):

```
tools: yt-dlp=Some("2026.10.21") ffmpeg=Some("8.0") ffprobe=Some("8.0") node=Some("22.14.0") deno=None
models: htdemucs (Separation), whisper-large-v3-turbo-q5 (Transcription)
project: 6 stages, htdemucs settings, reopened=true
selftest ok
```

`--selftest` no abre ninguna ventana ni usa la red: detecta herramientas, valida el catálogo de modelos y crea, guarda y reabre un proyecto temporal.

## 2. Primera canción

1. Arranca el ejecutable. Se abre la ventana principal con el panel de **Etapas** vacío.
2. En **YouTube** pega la URL del vídeo (`https://www.youtube.com/watch?v=...`). El enlace debe ser de un vídeo, no de una lista.
3. Deja vacío **Proyecto** o escribe la carpeta donde quieres guardar el proyecto (se crea un nombre con la marca de tiempo si lo dejas vacío).
4. Pulsa **Crear proyecto**. La app crea la estructura y muestra las seis etapas en gris (`Pending`).
5. Pulsa **Procesar todo**. El panel izquierdo va marcando cada etapa:

| Etapa | Qué hace | Salida |
|---|---|---|
| Ingest | Metadatos, audio, carátula y WAV mono 48 kHz | `source/`, `work/analysis.wav` |
| Separate | Separa voz e instrumental | `work/stems/` |
| Transcribe | Whisper sobre la voz | `work/transcript.json` |
| Pitch | F0, tempo y cuantización | `work/notes.json` |
| Align | Sílabas → notas | `work/song.json` |
| Export | Valida y escribe la carpeta | `output/<Artista> - <Título>/` |

Los tiempos típicos en CPU están en [`docs/performance.md`](docs/performance.md). La primera ejecución incluye cargar ~890 MB de modelos en memoria.

Si una etapa falla, se pone en rojo con el motivo, y el botón **Reintentar** vuelve a ejecutarla. **Cancelar** detiene el trabajo y deja la etapa en `Pending`.

### Idioma

Whisper detecta el idioma automáticamente. Para forzarlo, define `USDX_LANGUAGE=es` (o `en`) antes de arrancar: el pipeline usará ese idioma siempre y la silabificación usará el diccionario correspondiente. La app muestra el idioma detectado en `#LANGUAGE`.

### Canción de prueba (sin red)

Si quieres validar el pipeline sin descargar de YouTube, crea el proyecto a mano con un audio local:

```bash
USDX_PROJECT_DIR=/tmp/mi-cancion \
USDX_SOURCE_AUDIO=/ruta/voz.wav \
USDX_LANGUAGE=es \
USDX_HTDEMUCS_MODEL=$HOME/.local/share/usdx/models/htdemucs.onnx \
USDX_WHISPER_MODEL=$HOME/.local/share/usdx/models/ggml-large-v3-turbo-q5_0.bin \
cargo run -p usdx-application --example pipeline_smoke
```

El ejemplo codifica el audio a MP3, genera el WAV de análisis, ejecuta las etapas pendientes y termina con los tiempos por etapa y la ruta exportada. Es el mismo pipeline que usa la GUI.

## 3. Revisión en el editor

Cuando termina la alineación, el editor carga `work/song.json`.

- **Waveform:** clic para hacer *seek*. El deslizador **Zoom** cambia la ventana visible (2–120 s).
- **Piano-roll:** notas en verde, la seleccionada en naranja y las dudosas (`confianza < 0.55`) en rojo. Los `~` son continuaciones (melisma). Las líneas violetas marcan el final de cada frase.
- **Transporte:** play/pausa, parar, bucle de 4 s y cambio entre mezcla, voz e instrumental sin perder la posición.
- **Inspector:** mover, redimensionar, cambiar pitch, convertir en `~`, corregir el texto, crear o borrar notas.

| Tecla | Acción |
|---|---|
| `Espacio` | Play / pausa |
| `Shift+←` / `Shift+→` | ±2 s |
| `←` / `→` | Nota anterior / siguiente |
| `↑` / `↓` | ±1 semitono |
| `Supr` | Borrar nota |
| `Ctrl+Z` / `Ctrl+Y` | Deshacer / rehacer |

BPM y GAP están en la fila superior. Si el análisis generó candidatos de tempo, aparecen como botones «BPM / GAP» para comparar mitad y doble sin escribir números.

Las ediciones se autoguardan 2 s después del último cambio y hay 100 niveles de deshacer. Al cerrar con cambios pendientes aparece el diálogo de guardar/descartar/cancelar.

El flujo completo y sus límites están en [`docs/editor.md`](docs/editor.md).

## 4. Exportación

Pulsa **Exportar**. La app valida el documento y:

- si hay un error (por ejemplo una nota con texto vacío), selecciona la nota culpable, explica el motivo y no escribe nada;
- si solo hay avisos, exporta y los muestra;
- si todo es válido, crea:

```
output/<Artista> - <Título>/
├── <Artista> - <Título>.txt
├── audio.mp3
├── cover.jpg
└── video.mp4        (solo si activaste la descarga de vídeo)
```

El `.txt` incluye `#TITLE`, `#ARTIST`, `#LANGUAGE`, `#MP3`, `#COVER`, `#BPM`, `#GAP`, las frases con sus notas y el terminador `E`.

## 5. Copiar a UltraStar

1. Localiza la carpeta `songs/` de tu instalación:
   - **Windows:** `C:\Program Files (x86)\UltraStar WorldParty\songs\` o `%APPDATA%\UltraStar WorldParty\songs\`.
   - **macOS:** `~/Library/Application Support/UltraStar WorldParty/songs/`.
   - **Linux:** `~/.ultrastar-worldparty/songs/` o `~/.config/ultrastar-worldparty/songs/`.
2. Copia la carpeta completa `output/<Artista> - <Título>/` dentro de `songs/`. No copies solo el `.txt`: el `#MP3` es relativo a la carpeta.
3. Arranca UltraStar. La canción aparece como `<Artista> - <Título>`.
4. Si no aparece, revisa que el `.txt` tenga `#TITLE` y `#MP3`, y que `audio.mp3` esté junto a él.

Las canciones de referencia con las que se prueba el formato están en [`docs/performance.md`](docs/performance.md) y en la suite `crates/usdx/tests/reference_roundtrip.rs` (132 archivos, 63 656 notas).

## 6. Problemas frecuentes

| Síntoma | Causa y solución |
|---|---|
| «yt-dlp was not found in PATH» | Instala `yt-dlp` y reinicia la terminal/app para refrescar el `PATH` |
| «a JavaScript runtime is required» | Instala Node.js ≥ 18 o Deno |
| La etapa Separate falla con «model not found» | Revisa `USDX_HTDEMUCS_MODEL` o el campo **Modelo separación** |
| `shasum` no coincide | Descarga incompleta: borra el archivo y vuelve a bajarlo |
| La transcripción falla con «failed to encode» | Fallo intermitente de whisper.cpp en CPU; el motor reintenta hasta 5 veces. Si persiste, vuelve a pulsar **Reintentar** |
| No suena nada | No hay dispositivo de audio: el editor sigue funcionando en silencio |
| La canción se desincroniza al final | El formato no admite mapa de tempo; ajusta BPM/GAP o divide la canción |
| La letra sale mal | Corrige el texto en el inspector; la silabificación se recalcula al regenerar |

## 7. Desarrollo

```bash
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all --check
```

Suite de referencias UltraStar (necesita `refs/`):

```bash
USDX_REFS_DIR=refs cargo test -p usdx-format --test reference_roundtrip
```

Documentación técnica:

- [`docs/architecture.md`](docs/architecture.md) — capas y crates.
- [`docs/transcription.md`](docs/transcription.md) — Whisper, idiomas y filtros.
- [`docs/alignment.md`](docs/alignment.md) — silabificación y alineación.
- [`docs/editor.md`](docs/editor.md) — editor y atajos.
- [`docs/performance.md`](docs/performance.md) — tiempos medidos y aceptación.
- [`docs/licenses.md`](docs/licenses.md) — auditoría de licencias.

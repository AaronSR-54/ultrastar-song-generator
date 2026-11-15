# UltraStar Song Generator

Aplicación de escritorio que convierte una URL de YouTube en una carpeta de canción jugable en UltraStar Deluxe / USDX: letra silabificada, notas con pitch y duración, audio, carátula, vídeo y un editor para revisar el resultado antes de exportar.

La letra se busca primero en [LRCLIB](https://lrclib.net) (gratis, sin clave y con sincronización por palabra en muchas canciones). Si no hay una versión utilizable, la app la transcribe en local con whisper.cpp. La separación de voces (HTDemucs ONNX) y el análisis de pitch (pYIN) también corren en local: el audio nunca se sube a ningún servicio. Lo único que sale a la red es el título, el artista y la duración, para buscar la letra.

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

Los pesos no se incluyen en el repositorio ni en el binario. La app los busca en `USDX_MODELS_DIR`, `%TEMP%\usdx-models`, `~/.local/share/usdx/models` y junto al ejecutable; si faltan, los descarga y verifica sola.

| Modelo | Archivo | Tamaño | Cuándo se usa |
|---|---|---|---|
| HTDemucs 4-stem | `htdemucs.onnx` | 316 446 953 B | Siempre: separa la voz de la música |
| Whisper large-v3-turbo q5_0 | `ggml-large-v3-turbo-q5_0.bin` | 574 041 195 B | Solo si LRCLIB no tiene una letra con tiempos |

El primer procesamiento descarga HTDemucs (~302 MB). Whisper (~547 MB) se descarga aparte, y solo la primera vez que una canción no encuentra letra online.

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

Si ya los tienes en otra carpeta, apunta la app a ellos (opcional):

```bash
export USDX_HTDEMUCS_MODEL="$HOME/.local/share/usdx/models/htdemucs.onnx"
export USDX_WHISPER_MODEL="$HOME/.local/share/usdx/models/ggml-large-v3-turbo-q5_0.bin"
```

En PowerShell:

```powershell
$env:USDX_HTDEMUCS_MODEL = "$env:LOCALAPPDATA\usdx\models\htdemucs.onnx"
$env:USDX_WHISPER_MODEL  = "$env:LOCALAPPDATA\usdx\models\ggml-large-v3-turbo-q5_0.bin"
```

`USDX_OFFLINE=1` desactiva la búsqueda online de letra y usa siempre Whisper.

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

1. Arranca el ejecutable. La ventana abre en **Inicio** con un campo de URL y el botón **Procesar**.
2. Pega la URL del vídeo de YouTube (`https://www.youtube.com/watch?v=...`). Debe ser un vídeo, no una lista.
3. Pulsa **Procesar** (o Intro). No hay que crear ningún proyecto: la app lo prepara sola en su carpeta de datos (`projects/<id>`), ejecuta las etapas y exporta la canción.

| Etapa | Qué hace | Salida |
|---|---|---|
| Ingest | Metadatos, audio, carátula, vídeo y WAV mono 48 kHz | `source/`, `work/analysis.wav` |
| Separate | Separa voz e instrumental | `work/stems/` |
| Transcribe | Busca la letra en LRCLIB; si no hay, Whisper sobre la voz | `work/transcript.json` |
| Pitch | F0, tempo y cuantización | `work/notes.json` |
| Align | Sílabas → notas | `work/song.json` |
| Export | Valida y escribe la carpeta | `output/<Artista> - <Título>/` |

Los tiempos típicos en CPU están en [`docs/performance.md`](docs/performance.md). El primer procesamiento descarga HTDemucs (~302 MB); Whisper (~547 MB) solo se descarga la primera vez que una canción no encuentra letra online.

Si una etapa falla, la pantalla muestra el motivo. **Reintentar** continúa desde esa etapa y **Volver** permite pegar otro enlace. **Cancelar** detiene el trabajo y deja la etapa en `Pending`.

### Ajustes

El botón de ajustes permite elegir la carpeta de destino. Al terminar, la app copia allí la carpeta `Artista - Título` (sobrescribe archivos, no borra otros). Si no hay destino configurado, la canción queda en `output/` dentro del proyecto. La preferencia se guarda en `settings.json` y se aplica a las siguientes canciones.

### Idioma

La letra online incluye el idioma en sus metadatos; si falta o es incorrecto, la app lo deduce del texto (español e inglés, además de otros alfabetos). `USDX_LANGUAGE=es` (o `en`) fuerza el idioma y el diccionario de silabificación. `USDX_OFFLINE=1` omite la búsqueda online y usa siempre Whisper, que detecta el idioma automáticamente.

### Canción de prueba (sin red)

Si quieres validar el pipeline sin descargar de YouTube, crea el proyecto a mano con un audio local:

```bash
USDX_PROJECT_DIR=/tmp/mi-cancion \
USDX_SOURCE_AUDIO=/ruta/voz.wav \
USDX_LANGUAGE=es \
USDX_OFFLINE=1 \
USDX_HTDEMUCS_MODEL=$HOME/.local/share/usdx/models/htdemucs.onnx \
USDX_WHISPER_MODEL=$HOME/.local/share/usdx/models/ggml-large-v3-turbo-q5_0.bin \
cargo run -p usdx-application --example pipeline_smoke
```

El ejemplo codifica el audio a MP3, genera el WAV de análisis, ejecuta las etapas pendientes y termina con los tiempos por etapa y la ruta exportada. Es el mismo pipeline que usa la GUI.

## 3. Vista previa

Cuando termina la alineación, **Canción lista** ofrece **Vista previa**. Es una vista de solo lectura pensada para comprobar el sincronizado:

- **Frase actual:** letra completa de la frase, con la sílaba que suena resaltada. Cada sílaba es clicable para saltar a su instante.
- **Notas silabeadas:** ventana de 9 s que sigue la reproducción. Cada bloque es una sílaba; la frase actual se ilumina, la nota activa se rellena y la regla inferior marca los segundos.
- **Transporte:** play/pausa, detener, volver al principio y velocidad (`0,5×`, `0,75×`, `1×`, `1,25×`).
- **Barra de reproducción:** avanza con el audio y permite hacer *seek* arrastrando o haciendo clic.
- **Exportar:** vuelve a validar, exportar y copiar el documento a la carpeta de destino.

| Tecla | Acción |
|---|---|
| `Espacio` | Play / pausa |
| `←` / `→` | ±5 s |
| `Inicio` | Volver al principio |
| `Esc` | Volver a la pantalla anterior |

La vista previa no modifica el documento; para corregir notas hay que reprocesar la canción o usar las herramientas de línea de comandos.

### Barra de ventana

En **Ajustes → Barra de ventana** se elige cómo se presenta el marco:

| Modo | Comportamiento |
|---|---|
| **Sistema** | Barra nativa del sistema. En Windows 11 se tiñe con la paleta de la app (fondo, texto y borde) mediante DWM. |
| **Integrada** | Barra dibujada por la app con marca, estado local y controles de ventana; mismo color en todos los sistemas. |
| **Mínima** | Barra compacta de 28 px con la marca abreviada y los controles. |

En los modos integrados la ventana se puede arrastrar desde la barra y hacer doble clic para maximizar.

El flujo completo y sus límites están en [`docs/editor.md`](docs/editor.md).

## 4. Exportación

Al terminar el procesamiento la app ya ha exportado la canción y, si hay destino configurado, la ha copiado allí. Desde **Vista previa** puedes volver a exportar con **Exportar** (por ejemplo, después de cambiar la carpeta de destino). La app valida el documento y:

- si hay un error (por ejemplo una nota con texto vacío), selecciona la nota culpable, explica el motivo y no escribe nada;
- si solo hay avisos, exporta y los muestra;
- si todo es válido, crea:

```
output/<Artista> - <Título>/
├── <Artista> - <Título>.txt
├── audio.mp3
├── cover.jpg
└── video.mp4        (descargado por defecto)
```

El `.txt` incluye `#TITLE`, `#ARTIST`, `#LANGUAGE`, `#MP3`, `#COVER`, `#VIDEO` cuando el vídeo está
en la carpeta, `#BPM`, `#GAP`, las frases con sus notas y el terminador `E`.

## 5. Copiar a UltraStar

1. Localiza la carpeta `songs/` de tu instalación:
   - **Windows:** `C:\Program Files (x86)\UltraStar WorldParty\songs\` o `%APPDATA%\UltraStar WorldParty\songs\`.
   - **macOS:** `~/Library/Application Support/UltraStar WorldParty/songs/`.
   - **Linux:** `~/.ultrastar-worldparty/songs/` o `~/.config/ultrastar-worldparty/songs/`.
2. Abre **Ajustes** en la app y elige esa carpeta como destino. Desde ese momento cada canción se copia sola.
3. Si prefieres hacerlo a mano, copia la carpeta completa `output/<Artista> - <Título>/` dentro de `songs/`. No copies solo el `.txt`: el `#MP3` es relativo a la carpeta.
4. Arranca UltraStar. La canción aparece como `<Artista> - <Título>`.
5. Si no aparece, revisa que el `.txt` tenga `#TITLE` y `#MP3`, y que `audio.mp3` esté junto a él.

Las canciones de referencia con las que se prueba el formato están en [`docs/performance.md`](docs/performance.md) y en la suite `crates/usdx/tests/reference_roundtrip.rs` (132 archivos, 63 656 notas).

## 6. Problemas frecuentes

| Síntoma | Causa y solución |
|---|---|
| «yt-dlp was not found in PATH» | Instala `yt-dlp` y reinicia la terminal/app para refrescar el `PATH` |
| «a JavaScript runtime is required» | Instala Node.js ≥ 18 o Deno |
| La etapa Separate falla con «model not found» | Revisa `USDX_HTDEMUCS_MODEL` o deja que la app descargue el modelo |
| `shasum` no coincide | Descarga incompleta: borra el archivo y vuelve a bajarlo |
| «No se pudo descargar Whisper» | Solo se descarga al no encontrar letra online; revisa la conexión y pulsa **Reintentar** |
| La transcripción falla con «failed to encode» | Fallo intermitente de whisper.cpp en CPU; el motor reintenta hasta 5 veces. Si persiste, vuelve a pulsar **Reintentar** |
| No encuentra la letra online | La app usa Whisper automáticamente. Si el título de YouTube lleva sufijos (`(Official Video)`, `Lyric Video`), la búsqueda puede fallar: corrige el texto en el editor |
| La letra está desincronizada | La app corrige el desfase global cuando LRCLIB trae otra versión. Si aun así no cuadra, ajusta BPM/GAP en el editor |
| No suena nada | No hay dispositivo de audio: el editor sigue funcionando en silencio |
| La canción se desincroniza al final | El formato no admite mapa de tempo; ajusta BPM/GAP o divide la canción |
| La letra sale mal | Revisa la vista previa; si no cuadra, reprocesa la canción o edita `work/transcript.json` |

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

- [`docs/project-layout.md`](docs/project-layout.md) — capas, crates y estructura del proyecto.
- [`docs/transcription.md`](docs/transcription.md) — letra online, Whisper, idiomas y filtros.
- [`docs/alignment.md`](docs/alignment.md) — silabificación y alineación.
- [`docs/editor.md`](docs/editor.md) - vista previa, reproducción y barra de ventana.
- [`docs/performance.md`](docs/performance.md) — tiempos medidos y aceptación.
- [`docs/licenses.md`](docs/licenses.md) — auditoría de licencias.

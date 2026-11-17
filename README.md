# UltraStar Song Generator

Aplicación de escritorio que convierte una URL de YouTube en una carpeta de canción jugable en UltraStar Deluxe / USDX: letra silabificada, notas con pitch y duración, audio, carátula, vídeo y un editor para revisar el resultado antes de exportar.

La letra se busca primero en [LRCLIB](https://lrclib.net) (gratis, sin clave y con sincronización por palabra en muchas canciones). Si no hay una versión utilizable, la app la transcribe en local con whisper.cpp. La separación de voces (HTDemucs ONNX) y el análisis de pitch (pYIN) también corren en local: el audio nunca se sube a ningún servicio. Lo único que sale a la red es el título, el artista y la duración, para buscar la letra.

- **Plataformas:** Windows, macOS y Linux.
- **Formato:** `.txt` de UltraStar, una sola voz (sin duetos en el MVP).
- **Licencia:** MIT OR Apache-2.0. Detalles y dependencias en [`docs/licenses.md`](docs/licenses.md).

## Descarga

La [landing pública](https://ultrastar-song-generator.vercel.app) detecta el sistema y enlaza el ZIP correcto para Windows x86_64, macOS Apple Silicon o Linux x86_64. No hace falta instalador: descomprime el ZIP y ejecuta el binario.

Cada release adjunta los tres ZIP con su [`SHA256SUMS.txt`](https://github.com/AaronSR-54/ultrastar-song-generator/releases/latest/download/SHA256SUMS.txt). Si prefieres un instalador nativo, la misma release publica los `.msi`, `.dmg` y `.deb` con `SHA256SUMS-installers.txt`. Comprueba la suma antes de abrir el binario.

## 1. Instalación

### 1.1 Instalador

Cada release publica un instalador nativo para los tres sistemas. Es el camino
recomendado: la app queda en el menú de aplicaciones con su icono, sin compilar
nada.

| Sistema | Archivo |
|---|---|
| Windows 10/11 x64 | `UltraStar-Song-Generator-<versión>-windows-x64.msi` |
| macOS 11+ (Apple Silicon) | `UltraStar-Song-Generator-<versión>-macos-arm64.dmg` |
| Ubuntu 24.04 x86_64 | `ultrastar-song-generator_<versión>_amd64.deb` |

Descárgalo de <https://github.com/AaronSR-54/ultrastar-song-generator/releases>
y comprueba su hash contra `SHA256SUMS-installers.txt` antes de instalarlo.

- **Windows:** se instala por usuario, sin UAC ni permisos de administrador.
  Necesita el *Microsoft Visual C++ 2015-2022 Redistributable (x64)*; si falta,
  el propio instalador lo avisa con el enlace de descarga.
- **macOS:** requiere macOS 11 o superior y un Mac Apple Silicon; el bundle es
  arm64. Se instala arrastrando la app a *Applications*.
- **Linux:** `sudo apt install ./ultrastar-song-generator_<versión>_amd64.deb`.
  Deja `usdx-desktop` en el `PATH` y la entrada de menú.

Los artefactos **no están firmados** todavía: Windows muestra SmartScreen y
macOS bloquea la app con Gatekeeper la primera vez.
[`docs/install.md`](docs/install.md) explica cómo continuar de forma consciente,
cómo verificar la descarga y cómo desinstalar en cada sistema.

### 1.2 Herramientas externas

| Componente | Versión | Para qué |
|---|---|---|
| `yt-dlp` | 2025.09 o superior | Descargar de YouTube |
| Node.js ≥ 18 o Deno | cualquiera reciente | `yt-dlp` lo necesita para resolver YouTube |
| `ffmpeg` y `ffprobe` | 6.0 o superior | Convertir a WAV y leer duración |
| `espeak-ng` | 1.50 o superior | Fonemas IPA para medir dónde se canta cada sílaba (opcional) |
| Audio de salida | — | Reproducir en el editor (opcional) |

El instalador no incluye ninguna de estas herramientas: siguen siendo del
usuario. `yt-dlp`, `ffmpeg` y `ffprobe` deben estar en el `PATH`. La app las
detecta al arrancar, muestra su versión en **Ajustes** y, si falta alguna, la
etapa **Descarga** falla con un mensaje explícito antes de descargar nada. La
barra inferior solo muestra la versión de la app y la carpeta de destino.

Comandos de instalación por sistema en
[`docs/install.md#herramientas-externas`](docs/install.md#herramientas-externas).

### 1.3 Modelos

Los pesos no se incluyen en el repositorio ni en el instalador. La app los
descarga sola la primera vez que hacen falta y los guarda en su carpeta de datos;
no hay que descargarlos a mano.

| Modelo | Archivo | Tamaño | Cuándo se usa |
|---|---|---|---|
| HTDemucs 4-stem | `htdemucs.onnx` | 316 446 953 B | Siempre: separa la voz de la música |
| Whisper large-v3-turbo q5_0 | `ggml-large-v3-turbo-q5_0.bin` | 574 041 195 B | Solo si no hay letra online, no pegas la tuya y eliges transcribir |
| wav2vec2 phoneme CTC (espeak, int8) | `wav2vec2-espeak-ctc-int8.onnx` | 317 712 780 B | Siempre: mide dónde se canta cada sílaba |

El primer procesamiento descarga HTDemucs (~302 MB) y el alineador fonético
(~318 MB). Whisper (~547 MB) se descarga aparte, y solo si no hay letra online y
eliges transcribir en vez de pegarla. Las rutas, la descarga manual y las
variables `USDX_*_MODEL` están en [`docs/install.md#modelos`](docs/install.md#modelos)
y en [`docs/models.md`](docs/models.md).

`USDX_OFFLINE=1` desactiva la búsqueda online de letra: la app pide la letra o,
si eliges transcribir, usa Whisper.

> Los pesos de HTDemucs no están cubiertos por la licencia MIT del código de
> Demucs y se ofrecen para uso científico. Úsalos en local y no los
> redistribuyas. Si necesitas un producto comercial, sustitúyelos por pesos con
> licencia compatible.

> El alineador fonético son pesos de `facebook/wav2vec2-lv-60-espeak-cv-ft`
> (Apache-2.0), exportados a ONNX por `onnx-community`. Se pueden redistribuir
> conservando el aviso de licencia.

### 1.4 Comprobar la instalación

```bash
usdx-desktop --selftest
```

En Windows:

```powershell
& "$env:LOCALAPPDATA\Programs\UltraStar Song Generator\usdx-desktop.exe" --selftest
```

Salida esperada (las versiones pueden variar):

```
tools: yt-dlp=Some("2026.10.21") ffmpeg=Some("8.0") ffprobe=Some("8.0") node=Some("22.14.0") deno=None
models: htdemucs (Separation), whisper-large-v3-turbo-q5 (Transcription)
project: 6 stages, htdemucs settings, reopened=true
selftest ok
```

`--selftest` no abre ninguna ventana ni usa la red: detecta herramientas, valida
el catálogo de modelos y crea, guarda y reabre un proyecto temporal.

### 1.5 Compilar desde el código

Si prefieres trabajar en el proyecto en lugar de instalar la app, los requisitos
de compilación (Rust, CMake, LLVM, Visual Studio Build Tools) y su verificación
están en [`docs/setup.md`](docs/setup.md). El atajo una vez preparado el entorno:

```bash
git clone https://github.com/AaronSR-54/ultrastar-song-generator.git
cd ultrastar-song-generator
cargo build --release -p usdx-desktop
```

El ejecutable queda en `target/release/usdx-desktop` (`.exe` en Windows).

## 2. Primera canción

1. Arranca el ejecutable. La ventana abre en **Inicio** con un campo de URL y el botón **Generar**.
2. Pega la URL del vídeo de YouTube (`https://www.youtube.com/watch?v=...`). Debe ser un vídeo, no una lista.
3. Pulsa **Generar** (o Intro). La app descarga el audio y el vídeo, y muestra un formulario con los datos que ha podido extraer: título, artista, idioma, género, año y creador. Corrige lo que haga falta y pulsa **Confirmar y procesar**. No hay que crear ningún proyecto: la app lo prepara sola en su carpeta de datos (`projects/<id>`), ejecuta las etapas y exporta la canción.

| Etapa | Qué hace | Salida |
|---|---|---|
| Ingest | Metadatos, audio, carátula, vídeo y WAV mono 48 kHz | `source/`, `work/analysis.wav` |
| Separate | Separa voz e instrumental | `work/stems/` |
| Transcribe | Busca la letra en LRCLIB; si no hay, la pide o la transcribe con Whisper | `work/transcript.json` |
| Pitch | F0 y ataques, tempo y cuantización | `work/notes.json` |
| Align | Mide con fonemas dónde se canta cada sílaba → notas | `work/song.json` |
| Export | Valida y escribe la carpeta | `output/<Artista> - <Título>/` |

Los tiempos típicos en CPU están en [`docs/performance.md`](docs/performance.md). El primer procesamiento descarga HTDemucs (~302 MB) y el alineador fonético (~318 MB); Whisper (~547 MB) solo se descarga cuando una canción no encuentra letra online y decides transcribirla en vez de pegarla.

Si una etapa falla, la pantalla muestra el motivo. **Reintentar** continúa desde esa etapa y **Volver** permite pegar otro enlace. **Cancelar** detiene el trabajo y deja la etapa en `Pending`. Cuando no hay letra online, la app se detiene antes de descargar Whisper y muestra un campo opcional para pegarla, con dos botones: **Cancelar** y **Continuar**. El texto plano basta (una línea por verso) y también acepta LRC con `[mm:ss.xx]`; si el campo queda vacío, **Continuar** transcribe el audio.

### Ajustes

El botón de ajustes permite elegir la carpeta de destino. Al terminar, la app copia allí la carpeta `Artista - Título` (sobrescribe archivos, no borra otros). Si no hay destino configurado, la canción queda en `output/` dentro del proyecto.

**Por defecto** la app usa la carpeta `songs` de UltraStar Deluxe: lee `SongDir1` de `~/.ultrastardx/config.ini` (en Windows, `%APPDATA%\UltraStar Deluxe\config.ini`). Así, con el juego instalado, cada canción aparece sola en su lista sin configurar nada. En cuanto eliges una carpeta en Ajustes —o pulsas **Quitar destino**— manda tu elección y la detección automática deja de aplicarse. La preferencia se guarda en `settings.json` y se aplica a las siguientes canciones.

### Idioma

La letra online incluye el idioma en sus metadatos; si falta o es incorrecto, la app lo deduce del texto (español e inglés, además de otros alfabetos). `USDX_LANGUAGE=es` (o `en`) fuerza el idioma y el diccionario de silabificación. `USDX_OFFLINE=1` omite la búsqueda online: la app pide la letra y, si la pegas, la detecta del propio texto.

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

Cuando termina la alineación, **Canción lista** ofrece **Vista previa**. Sirve para comprobar el sincronizado y corregir las sílabas sin reprocesar:

- **Frase actual:** letra completa de la frase, con la sílaba que suena resaltada. Cada sílaba es clicable para saltar a su instante y un **doble clic** edita su texto en el sitio; arrastrando se dibuja un rectángulo que selecciona las sílabas que toca.
- **Notas silabeadas:** ventana de 9 s por defecto que sigue la reproducción. Cada bloque es una sílaba; la frase actual se ilumina, la nota activa se rellena y la regla inferior marca los segundos. Arrastrar pinta una goma de selección; la barra horizontal bajo la regla y la rueda desplazan la canción sin tocar el audio, y los botones **+** / **−** de la esquina ajustan el zoom (2–24 s).
- **Editor:** **No cantable**, **Rap**, **Normal** y **Borrar** se aplican a toda la selección.
- **Transporte:** play/pausa, detener, volver al principio y velocidad (`0,5×`, `0,75×`, `1×`, `1,25×`).
- **Barra de reproducción:** avanza con el audio y permite hacer *seek* arrastrando o haciendo clic.
- **Exportar:** vuelve a validar, exportar y copiar el documento a la carpeta de destino.

| Tecla | Acción |
|---|---|
| `Espacio` | Play / pausa |
| `←` / `→` | ±5 s |
| `Inicio` | Volver al principio |
| `F` / `R` / `N` | No cantable / rap / normal en la selección |
| `Supr` | Quitar las sílabas seleccionadas |
| `Ctrl+Z` / `Ctrl+Y` | Deshacer / rehacer |
| `Esc` | Volver a la pantalla anterior |

Los cambios se guardan solos a los 2 segundos; **Exportar** valida y reescribe la canción.

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

La app ya apunta a la carpeta de canciones del juego: al arrancar lee `SongDir1` de la configuración
de UltraStar Deluxe (`~/.ultrastardx/config.ini` en Linux y macOS; `%APPDATA%\UltraStar Deluxe\config.ini`
en Windows) y copia ahí cada canción terminada.

1. Si el juego está en otra ruta, abre **Ajustes** y elige su carpeta `songs/` con **Elegir**. Desde
   ese momento cada canción se copia sola.
2. Si prefieres hacerlo a mano, copia la carpeta completa `output/<Artista> - <Título>/` dentro de
   `songs/`. No copies solo el `.txt`: el `#MP3` es relativo a la carpeta.
3. Arranca UltraStar. La canción aparece como `<Artista> - <Título>`.
4. Si no aparece, revisa que el `.txt` tenga `#TITLE` y `#MP3`, y que `audio.mp3` esté junto a él.

Las canciones de referencia con las que se prueba el formato están en [`docs/performance.md`](docs/performance.md) y en la suite `crates/usdx/tests/reference_roundtrip.rs` (132 archivos, 63 656 notas).

## 6. Problemas frecuentes

| Síntoma | Causa y solución |
|---|---|
| «yt-dlp was not found in PATH» | Instala `yt-dlp` y reinicia la terminal/app para refrescar el `PATH` |
| «a JavaScript runtime is required» | Instala Node.js ≥ 18 o Deno |
| La etapa Separate falla con «model not found» | Revisa `USDX_HTDEMUCS_MODEL` o deja que la app descargue el modelo |
| `shasum` no coincide | Descarga incompleta: borra el archivo y vuelve a bajarlo |
| `HTTP Error 403` al descargar | YouTube lo devuelve de forma intermitente; la app reintenta una vez. Si se repite, actualiza `yt-dlp` |
| «No se pudo descargar Whisper» | Solo se descarga si eliges transcribir cuando no hay letra online; revisa la conexión y pulsa **Reintentar** |
| La transcripción falla con «failed to encode» | Fallo intermitente de whisper.cpp en CPU; el motor reintenta hasta 5 veces. Si persiste, vuelve a pulsar **Reintentar** |
| No encuentra la letra online | La app limpia sufijos (`(Official Video)`, `Lyric Video`), canales (`Queen Official`, `QueenVEVO`) y separadores (`-`, `–`, `—`). Si aun así falla, pega la letra en la pantalla que aparece: el texto plano es suficiente y se mide contra el audio |
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
- [`docs/code-layout.md`](docs/code-layout.md) — convenciones de módulos, visibilidad, tests y presupuesto de líneas.
- [`docs/transcription.md`](docs/transcription.md) — letra online, Whisper, idiomas y filtros.
- [`docs/alignment.md`](docs/alignment.md) — silabificación y alineación.
- [`docs/editor.md`](docs/editor.md) - vista previa, reproducción y barra de ventana.
- [`docs/performance.md`](docs/performance.md) — tiempos medidos y aceptación.
- [`docs/licenses.md`](docs/licenses.md) — auditoría de licencias.

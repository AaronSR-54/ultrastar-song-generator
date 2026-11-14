# Proposal

## Why

Hoy convertir una canción de YouTube en una canción jugable de UltraStar Deluxe exige encadenar a mano descarga, separación de voz, transcripción, detección de tono, silabificación, alineación y escritura del `.txt`. No existe una herramienta integrada, local y multiplataforma que haga ese trabajo de principio a fin y permita revisar el resultado. El proyecto parte de cero, pero cuenta con una copia real de UltraStar WorldParty y 132 canciones de referencia para validar el formato.

## What Changes

- Nueva app de escritorio multiplataforma en Rust (Windows, macOS, Linux) con GUI nativa `egui/eframe`, que convierte una URL de YouTube en una carpeta de canción UltraStar.
- Ingesta de medios: descarga de audio, vídeo y carátula con `yt-dlp` y `ffmpeg` gestionados por la app; extracción de metadatos (título, artista, duración) y audio WAV mono normalizado.
- Separación local de voz e instrumental con HTDemucs ejecutado sobre ONNX Runtime, sin Python en tiempo de ejecución.
- Transcripción local de la voz con Whisper (whisper.cpp) incluyendo detección de idioma y marcas temporales por palabra.
- Análisis de tono sobre la pista vocal (pYIN/SWIPE'), segmentación en notas, conversión a MIDI-60 y cuantización a la rejilla de beats de UltraStar.
- Estimación de `#BPM` y `#GAP`, con ajuste manual desde el editor.
- Silabificación para español e inglés y alineación de sílabas a notas mediante programación dinámica guiada por las marcas de Whisper y los onsets de las notas.
- Exportación de un `.txt` compatible con UltraStar Deluxe/USDX (cabeceras, notas `:`/`*`/`-`/`E`, pitch relativo a C4, BPM 4x, GAP en ms) junto al audio y la carátula.
- Editor de revisión: reproducción con seek, waveform, piano-roll de notas, edición de texto/tiempo/pitch, divisiones de frase, undo/redo y reexportación.
- Gestión local del ciclo de vida del proyecto: artefactos intermedios, reanudación, invalidación por hash y progreso/cancelación por etapa.
- Descarga y verificación de modelos locales con hash, selección de backend (CPU/GPU) y funcionamiento offline tras la primera preparación.

**Fuera de alcance del MVP:** duetos P1/P2, vídeo final sincronizado, edición multipista, publicación o distribución de canciones, apps móviles, uso de APIs en la nube y sincronización de letras ya existentes.

## Capabilities

### New Capabilities

- `song-projects`: ciclo de vida del proyecto de canción, estructura del workspace, estado de etapas, reanudación, invalidación de caché, progreso y cancelación.
- `media-ingestion`: validación de URL, descarga de audio/vídeo/carátula, metadatos y extracción de audio normalizado.
- `vocal-separation`: separación local de voz e instrumental con modelos ONNX y contrato de motor intercambiable.
- `pitch-analysis`: seguimiento de F0, segmentación de notas, mapeo a pitch UltraStar, estimación de tempo y cuantización.
- `lyrics-transcription`: transcripción local con Whisper, idioma y marcas temporales por palabra.
- `lyric-alignment`: silabificación es/en, alineación de sílabas a notas y puntuación de confianza.
- `ultrastar-format`: lectura, escritura y validación del formato `.txt` de UltraStar.
- `song-editor`: GUI de revisión y corrección con reproducción, waveform, piano-roll y exportación.
- `model-management`: catálogo, descarga, verificación, almacenamiento y selección de modelos y backends de inferencia.

### Modified Capabilities

Ninguna. El proyecto no tiene specs previas.

## Impact

- Proyecto nuevo: workspace Cargo con núcleo de dominio, adaptadores de inferencia, capa de aplicación y binario de escritorio `eframe`.
- Requisitos de sistema: Rust estable y toolchain MSVC en Windows; `yt-dlp` requiere un runtime JavaScript (Deno recomendado, Node compatible) para YouTube; `ffmpeg` para mux/extracción.
- Dependencias principales propuestas (versiones a fijar en implementación):
  - GUI y sistema: `eframe`/`egui` 0.37, `rfd` 0.18, `directories` 6, `tracing` + `tracing-subscriber`, `serde`/`serde_json`, `toml` 1, `thiserror` 2, `anyhow`, `tokio` 1.
  - Audio: `symphonia` 0.6, `rubato` 0.16, `realfft` 3.5, `hound` 3.5, `rodio` 0.23.
  - Inferencia: `whisper-rs` 0.16, `charon-audio` 0.1 (HTDemucs ONNX) sobre `ort` 2.0 rc, `pitch-core` 0.1 / `pyin` 1.2.
  - Texto: `hyphenation` 0.8 con diccionarios embebidos de español e inglés, `unicode-segmentation`, `unicode-normalization`.
  - Medios y modelos: `reqwest` 0.13, `sha2` 0.11, `image` 0.25, `lofty` 0.25.
- Riesgos de dependencias detectados:
  - `charon-audio` fija `ort = 2.0.0-rc.13` y `wav2vec2-rs` fija `ort = 2.0.0-rc.11`: no pueden convivir en el mismo grafo. El MVP no usará `wav2vec2-rs`; la alineación se hará con Whisper + notas. Una alineación acústica forzada opcional se aislará en un proceso auxiliar.
  - `yt-dlp` (crate) es GPL-3.0-only. Se invocará el binario `yt-dlp` por proceso en lugar de enlazar el crate, para no contaminar la licencia de la app.
  - Los pesos de HTDemucs no están cubiertos por la licencia MIT de Demucs y se distribuyen para uso científico; la app solo descargará modelos y el usuario asume el uso personal. Revisar antes de distribuir binarios con modelos incluidos.
  - `charon-audio` es joven (0.1.x, pocas descargas): se encapsula tras un trait y se cubre con pruebas de integración y un motor alternativo.
  - La app no puede garantizar calidad perfecta: la revisión humana en el editor es parte del flujo, no un extra.
- Legal/operativo: la descarga de contenido de YouTube está sujeta a sus términos; la herramienta se plantea para uso personal y no incluye funciones de publicación.

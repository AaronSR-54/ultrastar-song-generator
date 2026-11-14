# Design

## Context

Proyecto greenfield: no hay código, solo `refs/` con una instalación real de UltraStar WorldParty y `openspec/`. La máquina de desarrollo es Windows con `ffmpeg`, `yt-dlp`, Node y una GPU NVIDIA presentes, pero **sin Rust ni CMake instalados**. Ver proposal.md para la motivación.

Hechos del formato verificados sobre las 132 canciones de `refs/UltraStar WorldParty/songs/`:

- Cabeceras observadas: `#TITLE`, `#ARTIST`, `#MP3`, `#BPM`, `#GAP`, `#COVER`, `#VIDEO`, `#VIDEOGAP`, `#LANGUAGE`, `#YEAR`, `#GENRE`, `#CREATOR`, `#MEDLEYSTARTBEAT`, `#MEDLEYENDBEAT`, `#EDITION`, `#END`, `#START`, `#BACKGROUND`.
- Líneas de nota: `: startBeat length pitch text` (normal), `*` (dorada), `-` (corte de frase), `E` (fin). Las continuaciones usan `~` como texto.
- `pitch = 0` equivale a MIDI 60 (C4). Las notas doradas existen pero el generador del MVP no las creará; el parser las conserva.
- `#BPM` es cuatro veces el tempo musical: `beat_ms = 60000 / (BPM * 4)`, y `t_ms = GAP + beat * beat_ms`. Comprobado con dos referencias: "Adele - Hello" (`BPM 313,34`, último beat 5723 → 355 s de 366,9 s) y "a-ha - Take On Me" (`BPM 260`, último beat 3382 → 209 s de 215,8 s).
- `#BPM` usa coma decimal en las referencias (`313,34`); hay que aceptar coma y punto, y escribir de forma estable.
- El texto de una nota que inicia palabra lleva un espacio inicial (`: 62 6 -2  me`); las continuaciones no repiten letra.

## Goals / Non-Goals

**Goals:**

- Pipeline determinista y reanudable: cada etapa produce artefactos versionados y verificables.
- Cero Python y cero servicios externos en runtime; solo descarga inicial de modelos y binarios.
- Un único binario de escritorio por plataforma, con inferencia CPU como línea base.
- Precisión suficiente para que la revisión humana sea rápida: tiempos, pitch y sílabas editables en una sola pantalla.
- Formato de salida validado contra las referencias reales, no solo contra la documentación.

**Non-Goals:**

- Duetos, vídeo sincronizado, multipista, publicación y móvil (ver proposal).
- Garantizar letra o pitch perfectos sin revisión: el editor es parte del producto.
- Tempo variable dentro de una canción: el formato UltraStar usa un único `#BPM`.
- Entrenar modelos o fine-tuning.

## Decisions

### D1. Workspace hexagonal por capas

Se propone un workspace Cargo con dependencias en una sola dirección:

```
crates/
  domain/        # Entidades y reglas: Note, Syllable, SongDoc, Stage, Project
  ports/         # Traits: MediaSource, StemSeparator, Transcriber, PitchAnalyzer, ModelStore
  media/         # yt-dlp/ffmpeg por proceso, metadatos, carátula, audio
  ml/            # charon/ort, whisper-rs, pitch-core, post-proceso
  usdx/          # Parser, writer, validador y modelo de beats
  application/   # Orquestador de etapas, jobs, persistencia, invalidación
  desktop/       # eframe/egui, reproducción, editor
```

`domain` y `usdx` no dependen de GUI ni de inferencia, lo que permite tests rápidos y sustituir motores. `desktop` es el único binario del MVP; se añadirá un subcomando headless en `application` para pruebas E2E sin GUI.

**Alternativas:** un solo crate (más simple al inicio, pero acopla inferencia y UI y hace lentos los tests); Tauri (ver D2).

### D2. GUI nativa con eframe/egui 0.37

Se elige `eframe/egui` por: un solo binario, sin WebView ni Node, render inmediato adecuado para waveform/piano-roll, y compilación cruzada sencilla. El editor es fundamentalmente un lienzo custom; un frontend web no aporta ventaja suficiente para justificar Tauri.

**Alternativas:** Tauri 2 (mejor ecosistema de UI, pero WebView del sistema, más empaquetado y superficie de ataque); `iced` (retiene estado, pero menos maduro para edición densa y waveform custom).

### D3. `yt-dlp` y `ffmpeg` como procesos, nunca enlazados

El crate `yt-dlp` 2.10 es **GPL-3.0-only**; enlazarlo obligaría a licenciar toda la app como GPL. `rustube` está sin mantenimiento y rompe con cambios de YouTube. Por eso se invoca el binario `yt-dlp` con `tokio::process::Command` y argumentos explícitos, y `ffmpeg` para mux/extracción. La app detecta versiones, permite fijar rutas y actualiza el binario de forma controlada.

YouTube exige desde 2025 un runtime JavaScript externo para resolver desafíos (`--js-runtimes`). Se detecta Deno primero y Node después; en la máquina actual **Deno no está instalado y Node sí**. Si no hay ninguno, la etapa de descarga se bloquea con instrucciones. Para el MVP se exige un runtime del sistema; empaquetar Deno queda para la fase de distribución.

**Alternativas:** enlazar el crate GPL (descartado por licencia); implementar extractor propio (frágil e inviable).

### D4. Separación de voz local con HTDemucs ONNX

Se usa `charon-audio` 0.1.2, que implementa STFT/iSTFT, segmentación, overlap-add y normalización de Demucs 4.1 sobre `ort`, con salida de stems. Corre en CPU por defecto y permite CUDA/CoreML. No requiere Python y su licencia es MIT/Apache-2.0. Se encapsula tras `StemSeparator` para poder sustituirlo.

**Alternativas:** Demucs Python como subproceso (mejor calidad conocida, pero exige Python y un entorno reproducible); Spleeter (TF, anticuado); pipeline propio con `ort` (más control, pero replicar STFT/overlap-add es trabajo y riesgo).

**Decisión de versión:** `charon-audio` fija `ort = 2.0.0-rc.13`. Se fija esa versión en todo el workspace y se añade una comprobación de `cargo tree -d` para impedir dos versiones de `ort`.

### D5. Transcripción con whisper-rs 0.16

`whisper-rs` (Unlicense) enlaza whisper.cpp en proceso y expone timestamps por token con DTW. Se usa el stem vocal a 16 kHz mono. Modelo por defecto: variante multilingüe cuantizada (se decide entre `medium` y `large-v3-turbo` con una prueba comparativa; no cambia la arquitectura). Detección de idioma automática con override manual.

Para reducir alucinaciones: VAD/energía sobre el stem vocal, umbral de `no_speech`, filtro de repeticiones y descarte de segmentos en regiones sin voz. El resultado se guarda con texto, palabras, tiempos, idioma y confianza.

**Alternativas:** WhisperX (mejor alineación por palabra, pero Python y wav2vec2); API cloud (descartada por privacidad y coste); whisper.cpp CLI (proceso extra, sin ventaja).

**Prerrequisito de build:** whisper-rs necesita CMake y LLVM/clang además de MSVC en Windows. Hoy **CMake no está instalado**; es el primer bloqueo de toolchain.

### D6. Pitch y tempo en Rust puro

- **F0:** `pitch-core` 0.1.1 con pYIN por defecto (48 kHz, hop 20 ms, rango C2–C6). SWIPE' queda como alternativa configurable para voces con mucho ruido. Se evita CREPE (otro modelo y otra versión de runtime) y `aubio` (dependencia C y empaquetado multiplataforma complicado).
- **Notas:** filtro de mediana, histéresis de sonoridad, segmentación por cambio sostenido de al menos un semitono (≥60 ms), duración mínima 80 ms, unión de huecos <50 ms. Se conserva la altura media y la confianza por nota.
- **MIDI:** `midi = 69 + 12*log2(f/440)`; `pitch_usdx = round(midi) - 60`.
- **Tempo:** envolvente de onsets por flujo espectral (`realfft`), autocorrelación en el rango 60–180 BPM reales y fase por búsqueda de rejilla. Se muestran candidatos (incluidos mitad/doble) y el usuario confirma. `#BPM = 4 * bpm_real`; `#GAP` es el tiempo del beat 0.
- **Cuantización:** beat y longitud enteros, longitud mínima 1, resolución de conflictos por confianza. Si la deriva de tempo supera el umbral, se avisa; el formato no admite mapa de tempo.

**Alternativas:** `aubio` (tempo/onset listos, pero C); `bpm-analyzer`/`resonant-analysis` (menos control sobre fase y cuantización); librería Python (descartada).

### D7. Silabificación y alineación sin wav2vec2 en el MVP

`hyphenation` 0.8.4 con diccionarios embebidos de español e inglés (`Language::Spanish`, `Language::EnglishUS`) da una primera división ortográfica. Se añade una capa de reglas para diptongos, triptongos y hiatos del español, y se conserva el override manual. En inglés la división es ortográfica, no fonética; el editor cubre los casos dudosos.

La alineación es un DP monotónico entre sílabas y notas:

- Ancla de cada sílaba: interpolación lineal dentro del intervalo de su palabra según Whisper.
- Coste: distancia temporal al onset de la nota + adecuación de duración + continuidad de pitch + penalización por saltar sílaba (alta) o nota (baja, probable nota instrumental).
- Bandas por palabra para limitar el DP; complejidad O(S·N) con S≈500 y N≈700.
- Notas sobrantes dentro de una palabra → `~`; sílabas sobrantes → agrupación en la nota más cercana + marca de duda.
- Cortes de frase por silencio > umbral y por cambio de línea.

**Por qué no `wav2vec2-rs`:** fija `ort = 2.0.0-rc.11`, incompatible con `rc.13` de `charon-audio`; además es 0.1.0 y MPL-2.0. `forced-alignment` 0.5 usa candle y es solo inglés. La alineación acústica forzada multilingüe queda como mejora futura, aislada en un proceso auxiliar para no compartir grafo de dependencias.

### D8. Formato UltraStar propio y validado

Se implementa un parser/writer en `usdx` (el formato es pequeño y no hay crate mantenido que cubra notas, continuaciones y cabeceras). Reglas:

- Parser tolerante: BOM, CRLF/LF, etiquetas en mayúsculas/minúsculas, coma o punto decimal, `#END` y `E`, líneas desconocidas con warning.
- Writer determinista: UTF-8 sin BOM, LF, coma decimal, orden de cabeceras estable, nota final `E`.
- Texto: la primera nota de una frase no lleva espacio inicial; una nota que empieza palabra lleva un espacio; las continuaciones llevan `~`.
- Validación previa a exportar: artista/título/audio presentes, beats ≥0, longitud ≥1, pitch en rango, sin solapes, sin texto vacío salvo continuaciones, cortes de frase ordenados.
- Ida y vuelta semántica como test obligatorio.

### D9. Persistencia por proyecto, sin base de datos

Cada proyecto es una carpeta inspeccionable:

```
<project>/
  project.json                 # esquema versionado, ajustes, estados, hashes
  source/audio-source.m4a      # original de alta calidad
  source/cover.jpg
  source/video.mp4             # opcional
  work/audio-48k-mono.wav      # análisis
  work/stems/vocals.wav
  work/stems/instrumental.wav
  work/transcript.json
  work/pitch.json
  work/notes.json
  work/peaks.bin               # waveform multinivel
  output/<Artist> - <Title>/
```

`project.json` guarda `schema_version`, hashes SHA-256 de entradas y de configuración por etapa, y estados. La invalidación se basa en hashes, no en fechas. Escrituras atómicas (temporal + rename) y autoguardado con debounce.

**Alternativas:** SQLite (útil para biblioteca grande, innecesario para un proyecto a la vez); todo en un JSON (peor para artefactos grandes).

### D10. Orquestación de trabajos

Un runtime Tokio dedicado en un hilo, separado del hilo de UI de eframe. Las etapas de I/O usan Tokio; DSP e inferencia se ejecutan en `spawn_blocking`/rayon. La UI recibe eventos `JobEvent { stage, progress, message }` por canal y llama a `Context::request_repaint`. Cancelación con `CancellationToken` comprobado entre fragmentos. Solo una etapa pesada por proyecto; cola global de una tarea.

### D11. Modelos y backends

Catálogo compilado en la app (TOML) con id, propósito, URL HTTPS, tamaño, SHA-256, licencia, versión mínima y requisitos. Descarga con `reqwest` en streaming, `.part` + `Range`, verificación de hash y rename atómico. Almacenamiento en `ProjectDirs` (`%APPDATA%`, `~/Library/Application Support`, `~/.local/share`), con ruta configurable y comprobación de espacio. Consentimiento explícito antes de descargar.

Backend por defecto CPU. GPU opcional: CUDA (Windows/Linux) y CoreML (macOS) para `ort`; whisper.cpp con `cuda`/`metal`/`vulkan` según build. El MVP distribuye CPU; las builds GPU se documentan y se seleccionan por feature. Si el backend falla, se reintenta en CPU.

### D12. Reproducción y editor

`rodio` 0.23 con `try_seek` para reproducción, mezcla y selección de stem. La waveform se precalcula en una pirámide de picos (256/1024/4096 muestras por píxel) para dibujar sin decodificar en cada frame. Piano-roll en beats con zoom horizontal y sincronización con el cursor. Undo/redo por snapshots del `SongDoc` (pequeño) con límite de 100; autoguardado a los 2 s de inactividad. Atajos: espacio play/pausa, flechas para mover selección, `+`/`-` pitch, `Supr` borrar nota, `Ctrl+Z`/`Ctrl+Y`.

### D13. Auditoría de dependencias

| Crate / binario | Versión | Uso | Licencia | Notas |
|---|---|---|---|---|
| `eframe`, `egui` | 0.37 | GUI | MIT/Apache-2.0 | Línea base |
| `rfd` | 0.18 | Diálogos nativos | MIT | — |
| `directories` | 6 | Rutas de datos | MIT/Apache-2.0 | — |
| `tokio` | 1.54 | I/O, procesos, jobs | MIT | — |
| `serde`, `serde_json` | 1.0 | Persistencia | MIT/Apache-2.0 | — |
| `toml` | 1.2 | Configuración | MIT/Apache-2.0 | — |
| `thiserror`, `anyhow` | 2 / 1 | Errores | MIT/Apache-2.0 | — |
| `tracing`, `tracing-subscriber` | 0.1 / 0.3 | Logs | MIT | Sin telemetría |
| `symphonia` | 0.6 | Decodificación | MPL-2.0 | Copyleft a nivel de archivo; enlazable |
| `rubato` | 0.16 | Resampling | MIT | — |
| `realfft` | 3.5 | STFT/onsets | MIT/Apache-2.0 | — |
| `hound` | 3.5 | WAV | Apache-2.0/MIT | — |
| `rodio` | 0.23 | Reproducción | MIT/Apache-2.0 | Requiere ALSA en Linux |
| `whisper-rs` | 0.16 | ASR | Unlicense | Necesita CMake + LLVM/clang |
| `charon-audio` | 0.1.2 | Separación | MIT/Apache-2.0 | Joven; encapsulado |
| `ort` | 2.0.0-rc.13 | Runtime ONNX | MIT/Apache-2.0 | Fijado por charon-audio |
| `pitch-core` | 0.1.1 | F0 | MIT/Apache-2.0 | pYIN/SWIPE' |
| `pyin` | 1.2 | pYIN | MIT | vía pitch-core |
| `hyphenation` | 0.8.4 | Sílabas | MIT/Apache-2.0 | Patrones con licencias propias |
| `unicode-segmentation`, `unicode-normalization` | 1.13 / 0.1 | Texto | MIT/Apache-2.0 | — |
| `reqwest` | 0.13 | Descarga de modelos | MIT/Apache-2.0 | rustls |
| `sha2` | 0.11 | Integridad | MIT/Apache-2.0 | — |
| `image` | 0.25 | Carátula | MIT/Apache-2.0 | — |
| `lofty` | 0.25 | Metadatos | MIT | — |
| `yt-dlp` (binario) | última estable | Descarga | Unlicense | **No** se enlaza el crate GPL |
| `ffmpeg` (binario) | build LGPL | Mux/extracción | LGPL/GPL según build | No empaquetar build GPL |
| Deno o Node (runtime) | Deno ≥2 / Node LTS | Desafíos YouTube | MIT | Requisito de `yt-dlp` |
| Modelos Whisper | ggml multilingüe | ASR | MIT | Descarga del usuario |
| Pesos HTDemucs | ONNX | Separación | Uso científico, no MIT | No redistribuir; aviso legal |

**Rechazadas explícitamente:** crate `yt-dlp` (GPL-3.0-only), `wav2vec2-rs` (conflicto de `ort`), `forced-alignment` (solo inglés), `rustube` (sin mantenimiento), Demucs Python (requiere Python).

### D14. Prerrequisitos de build y empaquetado

- Rust estable (no instalado), VS Build Tools con C++ y Windows SDK, CMake y LLVM/clang (no instalados), Node o Deno (Node presente), `ffmpeg` (presente), ~5 GB libres (hay ~299 GB).
- Linux: `libasound2-dev`, `libxkbcommon-dev`, drivers OpenGL; macOS: Xcode CLT y CMake.
- Windows: soporte de rutas largas y nombres Unicode; macOS: normalización NFD al comparar nombres.
- Empaquetado: binario + modelos descargados aparte + aviso de licencias; no incluir `ffmpeg` GPL ni pesos Demucs por defecto.

## Risks / Trade-offs

- **Calidad de separación/ASR en canciones reales** → medir con un conjunto pequeño y diverso; el editor y los umbrales de confianza son la válvula de escape; nunca afirmar calidad perfecta.
- **`charon-audio` 0.1.x puede cambiar o fallar** → trait + versión fijada + tests de integración + posibilidad de motor alternativo (Demucs CLI) sin tocar dominio.
- **Conflicto de `ort` con futuras dependencias** → lockfile, `cargo tree -d` en CI, aislar alineación acústica en proceso aparte.
- **Tempo drift y formato de BPM único** → avisar de deriva; permitir BPM alto (más resolución); el usuario corrige. No prometer canciones con tempo variable.
- **Whisper alucina en instrumental** → VAD/energía, umbral de no-speech, filtro de repetición y descarte; el editor permite borrar líneas.
- **Sílabas inglesas ortográficas** → editor + confianza; no se promete fonética perfecta.
- **Licencia de pesos Demucs** → descarga con consentimiento, no redistribución, revisión legal antes de publicar.
- **Licencia de `ffmpeg`** → usar build LGPL y no bundlear GPL; documentar al usuario.
- **Rendimiento en CPU** → presupuesto: separación < 3 min para 4 min de audio en CPU moderna; transcripción < 2 min con modelo medio; pitch < 10 s; alineación < 1 s. Si no se cumple, degradar modelo, no bloquear UI.
- **Dependencia de YouTube** → yt-dlp se actualiza con frecuencia; permitir actualización y mostrar errores claros; no garantizar disponibilidad.
- **Privacidad** → todo local; no telemetría; las URLs y el audio no salen del equipo salvo la descarga de modelos/binarios.

## Migration Plan

Greenfield: no hay datos ni usuarios que migrar. El despliegue se hará por fases con el formato y el proyecto versionados desde el día uno:

1. Toolchain y workspace + fixtures de referencia.
2. `usdx` (parser/writer/validador) y store de proyecto.
3. Ingesta de medios.
4. Separación.
5. Pitch/tempo.
6. Transcripción.
7. Alineación y editor.
8. Empaquetado y pruebas de aceptación con UltraStar WorldParty.

Rollback: cada etapa es idempotente y escribe en `work/`; una versión nueva puede volver a la anterior mientras `schema_version` sea compatible. Los archivos de salida solo se escriben tras validación y confirmación, así que un fallo no destruye una exportación previa. No hay migración de datos de usuario en el MVP.

## Open Questions

- Tamaño y cuantización exactos de los modelos por defecto (medio vs large-v3-turbo; q5 vs q8): se decide con una comparativa de calidad/tiempo sobre 3–5 canciones; no cambia arquitectura ni specs.
- Estrategia de distribución de GPU (un binario con execution providers cargados en runtime vs builds separadas): se decide al empaquetar; CPU sigue siendo la línea base.
- Si se empaqueta Deno como sidecar o se exige instalación del sistema: el MVP exige sistema; empaquetarlo es una mejora de distribución.

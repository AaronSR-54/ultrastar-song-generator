# Licencias y cumplimiento

Este documento resume las licencias de todo lo que la app enlaza, ejecuta o descarga. No es asesoramiento legal: antes de publicar binarios, revisa la lista contra el `Cargo.lock` final.

## Licencia del proyecto

El código de este repositorio se distribuye bajo **MIT OR Apache-2.0**, salvo que un archivo indique otra cosa.

## Dependencias enlazadas (Rust)

| Componente | Uso | Licencia | Notas |
|---|---|---|---|
| `eframe`, `egui` | GUI | MIT OR Apache-2.0 | — |
| `rfd` | Diálogos nativos | MIT | — |
| `directories` | Rutas de datos | MIT OR Apache-2.0 | — |
| `tokio` | I/O, procesos, jobs | MIT | — |
| `serde`, `serde_json` | Persistencia | MIT OR Apache-2.0 | — |
| `toml` | Configuración | MIT OR Apache-2.0 | — |
| `thiserror`, `anyhow` | Errores | MIT OR Apache-2.0 | — |
| `tracing`, `tracing-subscriber` | Logs | MIT | Sin telemetría |
| `symphonia` | Decodificación de audio | MPL-2.0 | Copyleft a nivel de archivo; se puede enlazar sin relicenciar el resto |
| `rubato` | Remuestreo | MIT | — |
| `realfft` | FFT/onsets | MIT OR Apache-2.0 | — |
| `hound` | WAV | Apache-2.0 OR MIT | — |
| `rodio` | Reproducción | MIT OR Apache-2.0 | ALSA en Linux |
| `whisper-rs` | ASR (bindings) | Unlicense | whisper.cpp es MIT |
| `charon-audio` | Separación de stems | MIT OR Apache-2.0 | — |
| `ort` | Runtime ONNX | MIT OR Apache-2.0 | ONNX Runtime es MIT |
| `pitch-core` | F0 | MIT OR Apache-2.0 | — |
| `pyin` | pYIN | MIT | Vía `pitch-core` |
| `hyphenation` | Silabificación | MIT OR Apache-2.0 | Los patrones `hyph-utf8` conservan sus licencias |
| `unicode-segmentation`, `unicode-normalization` | Texto | MIT OR Apache-2.0 | — |
| `reqwest` | Descarga de modelos | MIT OR Apache-2.0 | Usar rustls |
| `sha2` | Integridad | MIT OR Apache-2.0 | — |
| `image` | Carátula | MIT OR Apache-2.0 | — |
| `lofty` | Metadatos | MIT | — |

## Herramientas externas (no enlazadas)

| Componente | Uso | Licencia | Regla |
|---|---|---|---|
| `yt-dlp` (binario) | Descarga de YouTube | Unlicense (dominio público) | Se ejecuta como proceso; **no** se enlaza el crate `yt-dlp` |
| crate `yt-dlp` | — | **GPL-3.0-only** | **Prohibido enlazarlo**: obligaría a relicenciar la app como GPL |
| `ffmpeg` | Mux/extracción de audio | LGPL-2.1+ o GPL según build | Distribuir solo builds **LGPL**; no empaquetar builds `--enable-gpl` |
| Deno | Runtime JS para YouTube | MIT | Dependencia del usuario o sidecar |
| Node.js | Runtime JS alternativo | MIT | Alternativa a Deno |
| ONNX Runtime | Ejecución de HTDemucs | MIT | Descargado como dependencia de `ort` |

## Modelos descargados

| Modelo | Uso | Licencia | Regla |
|---|---|---|---|
| Whisper GGML (multilingüe) | Transcripción | MIT | Se descarga desde el repositorio oficial |
| HTDemucs ONNX (4 stems) | Separación de voz | **No MIT**: pesos para uso científico | No redistribuir los pesos; descarga con consentimiento y aviso de uso personal |
| Modelos auxiliares de pitch | F0 | Según el modelo | Solo se usan modelos DSP sin pesos en el MVP |

Los pesos de Demucs no están cubiertos por la licencia MIT del código de Demucs. La app no los incluye en el repositorio ni en el instalador: el usuario los descarga y acepta el aviso. Si se quiere distribuir un producto comercial, hay que sustituirlos por pesos con licencia compatible o negociar una licencia.

## Reglas de cumplimiento

1. No añadir el crate `yt-dlp` a `Cargo.toml`. Usar el binario mediante `std::process`/`tokio::process`.
2. No empaquetar un `ffmpeg` compilado con `--enable-gpl` ni librerías GPL.
3. No incluir pesos de HTDemucs en el repositorio, en el instalador ni en los artefactos de release.
4. Mantener `symphonia` (MPL-2.0) como dependencia separada; no copiar su código a archivos propios.
5. Revisar las licencias de los patrones de `hyphenation` si se añaden idiomas nuevos.
6. Generar un archivo `THIRD_PARTY_NOTICES` con las licencias completas antes de publicar.

## Cómo auditar

```bash
# Árbol con licencias declaradas
cargo tree --workspace --format "{p} {l}"

# Buscar duplicados y dependencias inesperadas
cargo tree -d

# Revisar el lockfile completo
cargo metadata --format-version 1
```

Antes de cada release:

- [ ] `cargo tree --workspace --format "{p} {l}"` no muestra licencias incompatibles.
- [ ] `Cargo.lock` no contiene el crate `yt-dlp`.
- [ ] El instalador no incluye `ffmpeg` GPL ni pesos Demucs.
- [ ] `THIRD_PARTY_NOTICES` está actualizado.

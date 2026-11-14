# Rendimiento y aceptación

Resultados medidos en el equipo de desarrollo (Windows 11, Rust 1.100 MSVC, CPU de 8 núcleos, 32 GB de RAM, sin GPU) con builds **debug**. Las builds release son más rápidas; estas cifras sirven como cota conservadora.

## Presupuestos del diseño

| Etapa | Presupuesto (canción de 3–4 min) | Medido (audio corto) |
|---|---|---|
| Descarga + conversión | < 60 s | 5,6 s (19 s de audio) |
| Separación HTDemucs | < 180 s | 32,7 s (19 s de audio) |
| Transcripción Whisper | < 120 s | 8–16 s (10–19 s de audio) |
| Pitch + tempo | < 60 s | 26–49 s (10–19 s de audio) |
| Alineación | < 5 s | < 0,1 s |
| Exportación | < 2 s | < 0,1 s |

El coste de separación, transcripción y pitch crece de forma aproximadamente lineal con la duración del audio, así que una canción de 3,5 minutos se estima en 6–12 minutos en CPU (la separación domina). Es coherente con el presupuesto del diseño y queda por debajo de los 15 minutos del criterio de aceptación.

## Ejecuciones de aceptación

Comando usado (headless, sin GUI):

```bash
cargo run -p usdx-application --example pipeline_smoke
```

con `USDX_PROJECT_DIR`, `USDX_SOURCE_URL` o `USDX_SOURCE_AUDIO`, `USDX_LANGUAGE`, `USDX_HTDEMUCS_MODEL` y `USDX_WHISPER_MODEL`.

### Español (voz sintética, archivo local)

| Etapa | Tiempo |
|---|---|
| Ingest | omitido (archivo local, 10 s) |
| Separate | 20,6 s |
| Transcribe | 8,1 s |
| Pitch | 26,3 s |
| Align | < 0,1 s |
| Export | < 0,1 s |
| **Total** | **34,4 s** |

Resultado: `#LANGUAGE:es`, `#BPM:449,39`, 30 notas en 3 frases, 5 candidatos de tempo. Texto reconocido: «Hola, esta es una prueba de transcripción de voz en español… Cantamos una canción sencilla bajo la luna.»

Comprobaciones de la checklist:

- [x] Audio `audio.mp3` en la carpeta exportada.
- [x] Carátula `cover.jpg`.
- [x] Letra en español con sílabas y espacios de palabra correctos.
- [x] Notas con beat, longitud y pitch en rango.
- [x] Cortes de frase (`- inicio fin`) en las pausas.
- [x] Terminador `E`.

### Inglés (YouTube, `jNQXAC9IVRw`, 19 s)

| Etapa | Tiempo |
|---|---|
| Ingest (yt-dlp + ffmpeg + carátula) | 5,6 s |
| Separate | 32,7 s |
| Transcribe | 15,5 s (incluye un reintento interno por `-6`) |
| Pitch | 48,8 s |
| Align | < 0,1 s |
| Export | < 0,1 s |
| **Total** | **102,6 s** |

Resultado: `#LANGUAGE:en`, `#BPM:356,41`, 24 notas en 13 frases, carátula descargada (64 KB). Como el audio es habla y no canto, el pitch es ruidoso y el alignment agrupa palabras; es el comportamiento esperado y el editor permite corregirlo.

## Referencias UltraStar

```bash
USDX_REFS_DIR=refs cargo test -p usdx-format --test reference_roundtrip
```

Resultado: **132 archivos** (132 de una voz, 0 duetos), **63 656 notas**, todos parseados y reescritos con round-trip semántico exacto. El colector ignora los `README.txt` y licencias de la carpeta de referencias.

## Notas

- El reintento de Whisper (`MAX_ENCODE_ATTEMPTS = 5`) está incluido en los tiempos; un intento fallido cuesta unos 8 s.
- La primera ejecución incluye la carga de los modelos (547 MB de Whisper y 316 MB de HTDemucs) en memoria; el tiempo de carga está dentro de las cifras de cada etapa.
- El modo local del ejemplo codifica el audio a MP3 y genera el WAV de análisis antes de separar, para que el proyecto quede igual que tras un ingest real.

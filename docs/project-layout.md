# Estructura de un proyecto

Cada canción en preparación vive en su propia carpeta. Todo es inspeccionable con el explorador de archivos; no hay base de datos.

## Árbol

```
<project>/
  project.json
  source/
    audio-source.mp3
    cover.jpg
    video.mp4              # opcional
  work/
    audio-48k-mono.wav
    transcript.json
    pitch.json
    notes.json
    peaks.bin
    stems/
      vocals.wav
      instrumental.wav
  output/
    <Artist> - <Title>/
      <Artist> - <Title>.txt
      <Artist> - <Title>.mp3
      <Artist> - <Title>.jpg
```

Los nombres de `source/` y `work/` son estables y los define `ProjectPaths`. La carpeta `output/` solo se escribe después de validar y confirmar la exportación.

## project.json

Esquema versionado (`schema_version: 1`). Ejemplo real generado por `ProjectStore::create`:

```json
{
  "schema_version": 1,
  "id": "1a1d9fd4e19-my-song",
  "created_at_unix_ms": 1794658618905,
  "source": {
    "url": "https://www.youtube.com/watch?v=dQw4w9WgXcQ",
    "site": "YouTube",
    "title": "My Song",
    "artist": "Artist",
    "duration_ms": 215000,
    "thumbnail_url": null
  },
  "settings": {
    "language": null,
    "force_language": false,
    "download_video": false,
    "audio_quality": "best",
    "separation_model": "htdemucs",
    "whisper_model": "medium"
  },
  "stages": {
    "ingest": {
      "status": "pending",
      "input_hash": null,
      "config_hash": null,
      "error": null,
      "started_at_unix_ms": null,
      "finished_at_unix_ms": null,
      "metrics": {}
    },
    "separate": {
      "status": "pending",
      "input_hash": null,
      "config_hash": null,
      "error": null,
      "started_at_unix_ms": null,
      "finished_at_unix_ms": null,
      "metrics": {}
    },
    "transcribe": {
      "status": "pending",
      "input_hash": null,
      "config_hash": null,
      "error": null,
      "started_at_unix_ms": null,
      "finished_at_unix_ms": null,
      "metrics": {}
    },
    "pitch": {
      "status": "pending",
      "input_hash": null,
      "config_hash": null,
      "error": null,
      "started_at_unix_ms": null,
      "finished_at_unix_ms": null,
      "metrics": {}
    },
    "align": {
      "status": "pending",
      "input_hash": null,
      "config_hash": null,
      "error": null,
      "started_at_unix_ms": null,
      "finished_at_unix_ms": null,
      "metrics": {}
    },
    "export": {
      "status": "pending",
      "input_hash": null,
      "config_hash": null,
      "error": null,
      "started_at_unix_ms": null,
      "finished_at_unix_ms": null,
      "metrics": {}
    }
  }
}
```

`id` combina el instante de creación en hexadecimal con un slug del título. `created_at_unix_ms` y los timestamps de etapa son milisegundos Unix.

El test `docs_match_project.rs` extrae este bloque y comprueba que sus claves coinciden con las de un `project.json` recién generado.

## Estados de etapa

| Estado | Significado |
|---|---|
| `pending` | Pendiente o invalidada; se puede ejecutar |
| `running` | En ejecución |
| `done` | Completada |
| `failed` | Falló; conserva `error` y se puede reintentar |
| `skipped` | Omitida deliberadamente (por ejemplo, una etapa opcional) |

Transiciones permitidas:

- `pending → running → done`
- `running → failed`
- `running → pending` (cancelación)
- `pending → skipped`
- `done`, `failed` o `skipped → pending` (invalidación)
- Una etapa `running` no se puede invalidar.

## Etapas y dependencias

| Etapa | Depende de | Invalida al cambiar |
|---|---|---|
| `ingest` | URL y ajustes de descarga | ingest, separate, transcribe, pitch, align, export |
| `separate` | audio de `ingest` | separate, transcribe, pitch, align, export |
| `transcribe` | `vocals.wav` y idioma | transcribe, align, export |
| `pitch` | `vocals.wav` | pitch, align, export |
| `align` | transcripción + pitch | align, export |
| `export` | alineación + metadatos | export |

Cambiar la calidad del audio invalida todo el pipeline. Cambiar el idioma solo invalida transcripción, alineación y exportación; la separación y el pitch se reutilizan.

Cada etapa completada guarda `input_hash` y `config_hash` (SHA-256) para decidir si hace falta repetirla. Los hashes se calculan con `hash_file`, `hash_json` o `hash_bytes`.

## Escritura atómica y recuperación

`ProjectStore::save` escribe primero un temporal `.project.json.tmp`, hace `sync_all` y luego reemplaza el archivo:

- En Unix, `rename` es atómico.
- En Windows, el archivo actual se mueve a `.project.json.bak`, se renombra el temporal y se borra el backup. Si el reemplazo falla, se restaura el backup.
- Al abrir, si falta `project.json` pero existe `.project.json.bak`, se restaura el backup.
- Un temporal truncado por un corte nunca se usa como fuente.

El autoguardado usa `Autosave` con debounce de 2 s: las ediciones marcan el documento como sucio y se guarda cuando pasa el intervalo.

## Ejecución de trabajos

`JobRunner` crea un runtime Tokio con dos workers en un hilo dedicado y ejecuta una sola tarea pesada a la vez:

- `spawn` devuelve `JobError::Busy` si ya hay una tarea activa.
- Los trabajos reciben un `JobContext` con `is_cancelled`, `check_cancelled` y `report(fracción, mensaje)`.
- Los eventos `Progress` y `Finished` llegan por un canal que la UI consume.
- La cancelación es cooperativa: cada etapa comprueba el flag entre fragmentos.
- Al terminar, el flag de ocupado se libera incluso si el trabajo entra en pánico.

## Reanudación

`Project::next_stage()` devuelve la primera etapa `pending` o `failed` en orden de dependencia. Tras un fallo en `separate`, las etapas anteriores se conservan y el proyecto se reanuda exactamente ahí. Las etapas `skipped` se consideran completadas.

## Cómo inspeccionar un proyecto

```bash
cat <project>/project.json
# o, desde Rust:
cargo run -p usdx-application --example new_project -- /tmp/my-project
```

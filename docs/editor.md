# Editor de revisión

La app de escritorio (`crates/desktop`) abre proyectos, ejecuta las etapas del pipeline y permite revisar y corregir la canción generada antes de exportarla.

## Arranque

```bash
cargo run -p usdx-desktop
```

La app solo necesita un enlace de YouTube. Detecta las herramientas externas y los modelos por sí sola:

| Ajuste | Origen | Ejemplo |
|---|---|---|
| `yt-dlp` | `PATH` | 2025.09.26 o superior |
| `ffmpeg` / `ffprobe` | `PATH` | 6.0 o superior |
| Runtime JS para `yt-dlp` | `PATH` | Node 22 o Deno |
| Modelo de separación | `USDX_HTDEMUCS_MODEL` o descarga automática | `%TEMP%\usdx-models\htdemucs.onnx` |
| Modelo Whisper | `USDX_WHISPER_MODEL`, descarga bajo demanda | `%TEMP%\usdx-models\ggml-large-v3-turbo-q5_0.bin` |

Los modelos no se incluyen en el binario: la app descarga HTDemucs la primera vez y Whisper solo si la letra online no está disponible.

## Vista de proyecto y etapas

La ventana tiene tres pantallas: **Inicio** (URL + Procesar), **Procesando** (etapas y progreso) y **Listo** (abrir carpeta o revisar). La app crea el proyecto sola en su carpeta de datos; el engranaje abre **Ajustes**, donde se elige la carpeta de destino. Las seis etapas se muestran con su estado:

| Color | Estado | Acciones |
|---|---|---|
| gris | `Pending` | Ejecutar |
| amarillo | `Running` | Cancelar |
| verde | `Done` | Ejecutar de nuevo (recalcula) |
| rojo | `Failed` | Reintentar, o Ejecutar |
| violeta | `Skipped` | Ejecutar |

- **Procesar** recorre todas las etapas pendientes o fallidas en orden de dependencia y se detiene en el primer error.
- **Reintentar** continúa desde la etapa que falló; las anteriores no se repiten.
- **Cancelar** pide la cancelación cooperativa; la etapa vuelve a `Pending`, no a `Failed`.
- El mensaje de una etapa fallida se guarda en `project.json` y se muestra en la pantalla de error, junto con la acción de reintentar.

El pipeline escribe artefactos intermedios en `work/` (`transcript.json`, `notes.json`, `song.json`) y la etapa de exportación escribe `output/<Artista> - <Título>/`.

## Reproducción

`rodio` reproduce la mezcla (WAV de análisis) o cualquiera de los stems. La barra de transporte ofrece play/pausa, parar, cambio de stem y bucle. El bucle se activa en la posición actual y dura 4 s; volver a pulsar lo desactiva.

- Cambiar de stem conserva la posición y el estado de reproducción.
- Hacer clic en la waveform hace seek a ese instante; el cursor y el piano-roll se actualizan en el mismo frame.
- Si no hay dispositivo de audio, el editor funciona igual y muestra el aviso en la barra de estado.

## Waveform

La waveform se dibuja desde una pirámide de picos: nivel 0 guarda un mínimo/máximo cada 256 muestras (5,3 ms a 48 kHz) y cada nivel siguiente cuadruplica el bucket. Una canción de 6 minutos ocupa del orden de 67 000 picos en el nivel 0 y se construye en decenas de milisegundos, así que el zoom y el scroll no bloquean la UI.

El control **Zoom** cambia la ventana visible entre 2 y 120 s.

## Piano-roll

El piano-roll muestra:

- notas normales en verde, la seleccionada en naranja y las dudosas (`confidence < 0.55`) en rojo;
- continuaciones `~` como rectángulos huecos;
- cortes de frase como líneas verticales violetas al final de cada frase;
- la rejilla de beats (línea fuerte cada 4) y el cursor de reproducción.

Los cortes de frase (`- inicio fin`) vienen de la alineación: cambio de línea de la transcripción o silencio superior a 0.8 s.

## Edición

Selecciona una nota con el ratón (o con las flechas) y usa el inspector:

| Acción | Control |
|---|---|
| Mover | arrastrar la nota, o `←`/`→` |
| Redimensionar | botones `+ beat` / `- beat` |
| Cambiar pitch | `⏶`/`⏷` o `↑`/`↓` |
| Convertir en continuación | botón `~` |
| Cambiar texto | campo de texto del inspector |
| Crear nota | botón `+ nota` (al final de la frase seleccionada) |
| Borrar nota | botón `Borrar` o `Supr` |

Restricciones que aplica el estado del editor (y verifican los tests):

- Mover una nota la recorta al hueco entre la nota anterior y la siguiente; nunca invade a sus vecinas.
- Redimensionar respeta una longitud mínima de 1 beat y el inicio de la siguiente nota.
- Crear una nota en un hueco ocupado devuelve error y no modifica el documento.
- Borrar la última nota de una frase elimina la frase.
- El pitch se recorta al rango `PITCH_MIN..=PITCH_MAX` de UltraStar.

## Tempo y offset

La fila superior del editor expone `#BPM` (1–2000) y `#GAP` (−600 000 a 600 000 ms). Un valor inválido (0, negativo, NaN o fuera de rango) no se aplica y el documento conserva el anterior. Cambiar BPM o GAP solo mueve la rejilla y las notas en la vista previa: el audio no cambia de posición.

Si el proyecto tiene candidatos de tempo (`work/notes.json`), aparecen como botones «BPM / GAP». Al pulsar uno se aplican los dos valores a la vez, lo que permite comparar mitad/doble sin editar números a mano.

## Deshacer, autoguardado y cierre

- Cada edición guarda un snapshot del `SongDoc`. El historial conserva las últimas 100 operaciones (`UNDO_LIMIT`).
- Deshacer y rehacer restauran beats, longitud, pitch, texto y tipo exactos; rehacer aplica el cambio una sola vez.
- El documento se marca como sucio al editar y se autoguarda 2 s después de la última edición (`AUTOSAVE_DELAY`) en `work/song.json`.
- Al intentar cerrar con cambios pendientes aparece un diálogo con **Guardar y cerrar**, **Descartar** y **Cancelar**. Con el documento limpio la ventana se cierra directamente.

## Exportación

El botón **Exportar** valida el documento con el validador de `usdx-format`:

- si hay un error, el editor selecciona la frase y la nota culpables, muestra el motivo y no escribe nada (por ejemplo, una nota con texto vacío);
- si solo hay avisos, exporta y los muestra en la barra de estado;
- si todo es válido, crea `output/<Artista> - <Título>/` con el `.txt`, `audio.mp3`, `cover.jpg` y, si existe, `video.mp4`.

## Atajos

| Tecla | Acción |
|---|---|
| `Espacio` | Play / pausa |
| `Shift+←` / `Shift+→` | Retroceder / avanzar 2 s |
| `←` / `→` | Nota anterior / siguiente |
| `↑` / `↓` | Subir / bajar un semitono |
| `Supr` | Borrar la nota seleccionada |
| `Ctrl+Z` | Deshacer |
| `Ctrl+Y` | Rehacer |

Los atajos se ignoran mientras un campo de texto tiene el foco.

## Checklist manual de revisión

1. Crear un proyecto con una URL y pulsar **Procesar todo**.
2. Comprobar que las seis etapas acaban en verde y que `output/` contiene `.txt`, audio y carátula.
3. Reproducir la canción: play/pausa, clic en la waveform y bucle de 4 s.
4. Cambiar entre mezcla, voz e instrumental: la posición no cambia.
5. Seleccionar una nota dudosa (roja), corregir el texto y comprobar que el resaltado desaparece tras regenerar o editar.
6. Arrastrar una nota contra la anterior y verificar que se detiene en el borde.
7. Cambiar `#BPM` a 0: el valor no se aplica y aparece el rango válido.
8. Deshacer y rehacer un movimiento: la nota vuelve y se reaplica exactamente una vez.
9. Editar, esperar 2 s y comprobar que `work/song.json` cambia sin pulsar Guardar.
10. Borrar el texto de una nota y pulsar **Exportar**: se selecciona esa nota y se muestra el motivo.
11. Corregir el texto y exportar: el `.txt` se escribe y la ruta aparece en la barra de estado.
12. Con cambios pendientes, cerrar la ventana: aparece el diálogo de guardar/descartar/cancelar.

## Límites conocidos

- Una sola voz; duetos y multipista quedan fuera del MVP.
- El tempo es constante: no hay mapa de tempo, así que si el BPM estimado deriva, la rejilla se desvía hacia el final.
- La reproducción decodifica el archivo al cambiar de stem; en canciones muy largas el cambio puede tardar un instante.
- La silabificación inglesa es ortográfica; las sílabas dudosas se corrigen a mano en el inspector.

# Transcripción de la letra

Extrae la letra cantada del stem vocal con Whisper en local. No se sube audio a ningún servicio: `whisper.cpp` se enlaza en proceso mediante `whisper-rs` y el único tráfico de red de la app es la descarga de modelos y de la canción.

## Modelo

| Campo | Valor |
|---|---|
| Id de catálogo | `whisper-large-v3-turbo-q5` |
| Archivo | `ggml-large-v3-turbo-q5_0.bin` |
| Tamaño | 574 041 195 bytes (547 MiB) |
| SHA-256 | `394221709cd5ad1f40c46e6031ca61bce88931e6e088c188294c6d5a55ffa7e2` |
| Licencia | MIT |
| Idiomas | 100 idiomas |
| Entrada | WAV mono a 16 kHz |

El catálogo (`crates/application/assets/models.toml`) es la única fuente del tamaño y del hash. La app muestra el id, el tamaño, la licencia y la lista de idiomas desde `TranscriptionCapabilities`; el test `transcription_docs.rs` compara este documento con esa estructura para que no se desincronicen.

El modelo se descarga bajo demanda desde el repositorio oficial de `whisper.cpp` y se verifica antes de usarlo. La app no incluye pesos en el repositorio ni en el instalador.

## Idiomas

Whisper large-v3-turbo es multilingüe. La app expone la lista completa mediante `whisper_languages()` (código ISO-639-1 y nombre). Los idiomas validados de extremo a extremo para silabificación son:

| Código | Idioma | Silabificación |
|---|---|---|
| `es` | Español | Sí |
| `en` | Inglés | Sí |

El resto de idiomas se transcriben, pero la revisión y la silabificación usan reglas genéricas hasta que se añada su diccionario. La detección automática se ejecuta antes de la transcripción; el usuario puede fijar el idioma antes o después y volver a transcribir. Cambiar el idioma solo invalida transcripción, alineación y exportación.

## Flujo

1. Se lee `work/stems/vocals.wav`, se convierte a mono y se remuestrea a 16 kHz con `charon-audio`.
2. `WhisperContext::create_state` + `pcm_to_mel` preparan el audio.
3. Si no hay idioma forzado, `lang_detect` devuelve el idioma y su probabilidad. Un idioma desconocido se rechaza con `UnsupportedLanguage` antes de inferir.
4. `full` ejecuta Whisper con `token_timestamps`, `no_context` y el umbral de no-speech del filtro.
5. Los tokens se agrupan en palabras: un token que empieza por espacio inicia palabra; la puntuación se une a la palabra anterior.
6. Cada palabra recibe inicio, fin y confianza media de sus tokens. Los tiempos se fuerzan a ser monótonos y cada palabra dura al menos 20 ms.
7. Se aplica el filtro de alucinaciones y se construye el `Transcript`.

La cancelación se propaga a `whisper.cpp` con `set_abort_callback_safe`; el resultado parcial se descarta y la etapa vuelve a `pending`.

Si `whisper.cpp` devuelve el error transitorio de codificación (`-6`, «failed to encode»), el motor crea un estado nuevo y repite la inferencia una vez (`MAX_ENCODE_ATTEMPTS = 2`). Es un fallo intermitente conocido de la codificación en CPU que no depende del audio ni de la memoria disponible; tratarlo como fatal abortaría canciones enteras al azar. Cualquier otro error (o un segundo `-6`) se propaga como `TranscriptionError::Failed`.

## Marcas temporales

Los tiempos de token de whisper.cpp vienen en centisegundos; se convierten a segundos dividiendo entre 100. La garantía del formato es:

- `word.start_s >= previous_word.end_s`.
- `word.end_s > word.start_s`.
- `line.start_s` es el inicio de su primera palabra y `line.end_s` el fin de la última.

Una palabra con `confidence < 0.55` se marca `low_confidence` y el editor la resalta. El umbral es configurable en `TranscriptionOptions`.

## Filtro de alucinaciones

Whisper inventa texto en pasajes instrumentales. Antes de aceptar un segmento se comprueba:

| Regla | Umbral por defecto |
|---|---|
| Energía RMS media del segmento | ≥ 0.004 |
| Fracción de tramas con periodicidad vocal | ≥ 0.10 |
| Probabilidad de no-speech de Whisper | ≤ 0.60 |
| Repeticiones consecutivas de la misma línea | ≤ 2 |
| Frases de relleno conocidas (`thanks for watching`, `subtítulos por…`) | Se descartan |

La sonoridad se mide con autocorrelación en tramas de 40 ms sobre la señal decimada a 8 kHz, buscando periodicidad entre 80 y 1000 Hz. Un tono sostenido es periódico; el ruido instrumental no. Los segmentos descartados se cuentan por motivo en `DiscardReport` y el total se guarda en `Transcript::discarded_segments`; la UI puede mostrar «N líneas descartadas».

Si el stem vocal está vacío o casi vacío, la separación ya avisa con `VocalEnergy::is_voice_missing`; en ese caso la transcripción puede producir cero líneas y el editor permite escribir la letra a mano.

## Edición antes de alinear

El editor puede:

- **Corregir una palabra**: conserva `start_s` y `end_s` y borra solo las sílabas de esa palabra.
- **Borrar una palabra**: si era la última de la línea, la línea desaparece.
- **Unir una línea con la siguiente**: conserva todas las palabras y recalcula los límites de la línea.
- **Separar una línea por una palabra**: conserva los tiempos de cada palabra y reparte las palabras en dos líneas.
- **Borrar una línea**: sus palabras dejan de participar en la alineación y en la exportación.

`reapply_syllables` recorre solo las palabras cuya lista de sílabas está vacía. Tras corregir una palabra, el silabificador se invoca una única vez para esa palabra; el test `correcting_a_word_preserves_times_and_only_re_syllabifies_it` lo verifica con un silabificador falso.

## Límites y rendimiento

- Un modelo cuantizado q5 necesita ~1 GB de RAM además del audio.
- En CPU, la velocidad depende de los núcleos; `TranscriptionOptions::threads` usa hasta 8 hilos. Con audio de 10 s y el modelo turbo, la validación local tarda del orden de decenas de segundos; para una canción completa conviene medir y, si no se cumple el presupuesto del diseño, usar un modelo menor.
- La app no traduce: `translate = false` por defecto.
- No hay límite duro de duración; el coste es lineal con el audio. Audios de menos de 100 ms pueden no producir palabras.
- La transcripción asume una sola voz. En duetos, el MVP solo procesa la mezcla y el usuario debe corregir el texto.

## Verificación

Tests unitarios, sin modelo ni red:

```bash
cargo test -p usdx-ml --lib transcription
cargo test -p usdx-ml --lib transcript_edit
cargo test -p usdx-ml --lib hallucination
```

Tests reales, opcionales, con modelo y dos WAV de voz (cualquier grabación propia o fixture CC0, por ejemplo de Common Voice):

```bash
export USDX_WHISPER_MODEL=/ruta/ggml-large-v3-turbo-q5_0.bin
export USDX_WHISPER_AUDIO_ES=/ruta/voz-es.wav
export USDX_WHISPER_AUDIO_EN=/ruta/voz-en.wav
cargo test -p usdx-ml --test transcription -- --ignored --nocapture
```

Los WAV no se guardan en el repositorio. La validación local de este cambio usó voz sintetizada con las voces del sistema (Helena es-ES y Zira en-US); el test acepta cualquier WAV de voz con licencia compatible.

La prueba real (`real_model_end_to_end`, marcada `#[ignore]`) exige que el español se detecte como `es` con confianza > 0.3, al menos cinco palabras, texto reconocible y tiempos monótonos en ambos idiomas.

## Privacidad

- `whisper-rs` no abre conexiones de red.
- La única red del pipeline es `yt-dlp` (descarga) y la descarga de modelos.
- El audio, el transcript y los modelos permanecen en el equipo del usuario.

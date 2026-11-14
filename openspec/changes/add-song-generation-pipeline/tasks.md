# Tasks

## 1. Toolchain y esqueleto del workspace

- [x] 1.1 Instalar Rust estable con toolchain MSVC, CMake y LLVM/clang; verificar con `rustc --version`, `cargo --version`, `cmake --version` y una compilación trivial de whisper-rs.
- [x] 1.2 Crear el workspace con `domain`, `ports`, `media`, `ml`, `usdx`, `application` y `desktop`; verificar con `cargo check --workspace`.
- [x] 1.3 Fijar `ort = 2.0.0-rc.13` y generar `Cargo.lock`; verificar que `cargo tree -d -i ort` muestra una sola versión.
- [ ] 1.4 Añadir CI de `cargo fmt --check`, `cargo clippy -- -D warnings` y `cargo test --workspace`; verificar que el workflow pasa en un commit de prueba.
- [ ] 1.5 Documentar prerrequisitos de build por sistema operativo en `docs/setup.md`; verificar siguiendo las instrucciones en una carpeta limpia.
- [ ] 1.6 Crear `docs/licenses.md` con la matriz de licencias del diseño; verificar que incluye `yt-dlp` GPL, `ffmpeg` LGPL/GPL y pesos Demucs.

## 2. Núcleo del formato UltraStar

- [ ] 2.1 Implementar las entidades `SongDoc`, `Note`, `Phrase`, `Syllable` y `SongHeader` con serialización; verificar con tests de igualdad y valores por defecto.
- [ ] 2.2 Implementar el parser tolerante de `.txt` (BOM, CRLF/LF, coma/punto, etiquetas, `E`, `#END`, secciones `P1`/`P2`); verificar con tests unitarios por caso.
- [ ] 2.3 Implementar el writer determinista (UTF-8 sin BOM, LF, coma decimal, espacio inicial de palabra, `~`, cortes `-`, `E`); verificar con snapshots de archivos esperados.
- [ ] 2.4 Implementar las conversiones `beat↔ms` con `t = GAP + beat*60000/(BPM*4)` y `pitch = round(midi)-60`; verificar con los casos Adele Hello y Take On Me calculados en el diseño.
- [ ] 2.5 Implementar el validador previo a exportar (cabeceras, beats, longitud, pitch, solapes, texto vacío, frases); verificar con un test por regla y un archivo válido de control.
- [ ] 2.6 Añadir test de ida y vuelta sobre las referencias: parsear los 132 `.txt`, contar notas y comparar semánticamente los de una sola voz; verificar con `USDX_REFS_DIR` apuntando a `refs/`.
- [ ] 2.7 Documentar el subconjunto del formato soportado y las diferencias con duetos en `docs/usdx-format.md`; verificar que los ejemplos del documento se parsean en un test.

## 3. Proyecto, persistencia y orquestación

- [ ] 3.1 Implementar `project.json` versionado y la estructura `source/`, `work/`, `work/stems/`, `output/`; verificar creando, cerrando y reabriendo un proyecto de prueba.
- [ ] 3.2 Implementar escrituras atómicas y autoguardado con debounce; verificar que un corte simulado no deja JSON truncado.
- [ ] 3.3 Implementar la máquina de estados de etapa y el hash de entradas/configuración; verificar la matriz de invalidación del diseño con tests.
- [ ] 3.4 Implementar el runner de trabajos con Tokio en hilo aparte, eventos de progreso, cancelación y una sola tarea pesada; verificar con tests de doble ejecución y cancelación.
- [ ] 3.5 Verificar reanudación: simular fallo en separación, reabrir y comprobar que solo se repite desde esa etapa con un test de integración.
- [ ] 3.6 Documentar el esquema de proyecto y la política de invalidación en `docs/project-layout.md`; verificar que el documento coincide con un `project.json` real generado.

## 4. Ingesta de medios

- [ ] 4.1 Implementar la detección de `yt-dlp`, `ffmpeg` y runtime JavaScript (Deno/Node) con versiones y errores accionables; verificar con rutas válidas, ausentes y versiones antiguas.
- [ ] 4.2 Implementar la obtención de metadatos (`--dump-single-json`) y la sanitización de nombres portables; verificar con títulos con `:`, `/`, `?`, emoji y acentos.
- [ ] 4.3 Implementar la descarga de audio de alta calidad y la generación del WAV mono 48 kHz; verificar duración, canales y frecuencia contra el original.
- [ ] 4.4 Implementar la descarga y recorte cuadrado de carátula, y la sustitución por imagen local; verificar con una miniatura y una imagen inválida.
- [ ] 4.5 Implementar la descarga opcional de vídeo y su omisión ante ausencia o desactivación; verificar ambos caminos sin fallar el proyecto.
- [ ] 4.6 Implementar el adaptador de procesos con timeouts, cancelación y limpieza de `.part`; verificar con scripts falsos que simulan éxito, error y cancelación.
- [ ] 4.7 Documentar la dependencia de Deno/Node y los errores típicos de YouTube en `docs/media-ingestion.md`; verificar ejecutando una descarga real de prueba y registrando el resultado.

## 5. Gestión de modelos

- [ ] 5.1 Implementar el catálogo TOML compilado (id, propósito, URL, tamaño, SHA-256, licencia, requisitos); verificar que carga y valida todos los campos.
- [ ] 5.2 Implementar descarga en streaming con `.part`, reanudación por `Range`, verificación SHA-256 y rename atómico; verificar con un servidor HTTP local que corta la conexión.
- [ ] 5.3 Implementar el almacenamiento en `ProjectDirs`, cambio de carpeta, comprobación de espacio y eliminación por modelo; verificar con tests de rutas y un caso de espacio insuficiente simulado.
- [ ] 5.4 Implementar la detección de hardware y la selección CPU/GPU con degradación a CPU; verificar con backend falso que falla al inicializar.
- [ ] 5.5 Añadir tests de catálogo, reanudación, hash incorrecto y offline; verificar que ninguno usa red real.
- [ ] 5.6 Documentar el consentimiento, la licencia de cada modelo y el modo offline en `docs/models.md`; verificar que la app muestra el mismo texto que el documento.

## 6. Separación de voz

- [ ] 6.1 Definir el trait `StemSeparator` y el adaptador `charon-audio` con carga de modelo y configuración; verificar con un modelo de prueba y un audio sintético.
- [ ] 6.2 Generar `vocals.wav` e `instrumental.wav` con la misma frecuencia, canales y duración que la entrada; verificar longitudes muestra a muestra y suma aproximada a la mezcla.
- [ ] 6.3 Implementar progreso por fragmentos, cancelación y reintento CPU ante OOM; verificar con un backend simulado que agota memoria y con cancelación a mitad.
- [ ] 6.4 Implementar la medición de energía vocal y el aviso de voz insuficiente; verificar con un audio instrumental y con voz sintética.
- [ ] 6.5 Añadir tests de integración con una mezcla sintética (voz + tonos) y métricas mínimas de separación; verificar que el test es determinista.
- [ ] 6.6 Documentar el motor, límites de memoria y licencia de pesos en `docs/separation.md`; verificar que las cifras coinciden con una ejecución real.

## 7. Análisis de pitch y tempo

- [ ] 7.1 Implementar el pipeline de F0 con `pitch-core`/pYIN a 48 kHz y hop 20 ms, con sonoridad y confianza; verificar con tonos puros de 220, 440 y 880 Hz.
- [ ] 7.2 Implementar la segmentación en notas con mediana, histéresis, duración mínima y unión de huecos; verificar que un vibrato sintético no se fragmenta y un cambio de semitono sí.
- [ ] 7.3 Implementar la conversión a pitch UltraStar y el recorte de rango con advertencia; verificar 440 Hz → 9 y los límites.
- [ ] 7.4 Implementar la estimación de tempo, candidatos mitad/doble, fase y `#GAP`; verificar con clics sintéticos a 80, 100 y 120 BPM.
- [ ] 7.5 Implementar la cuantización a beats, longitud mínima 1 y resolución de solapes por confianza; verificar con notas solapadas y notas más cortas que un beat.
- [ ] 7.6 Añadir tests de reproducibilidad: dos ejecuciones con la misma entrada y configuración producen las mismas notas; verificar comparando JSON byte a byte.
- [ ] 7.7 Documentar parámetros y ajuste manual de BPM/GAP en `docs/pitch-and-tempo.md`; verificar que un usuario puede reproducir el ejemplo del documento con un audio de prueba.

## 8. Transcripción de letra

- [ ] 8.1 Integrar `whisper-rs`, carga de modelo multilingüe y ejecución sobre el stem vocal a 16 kHz; verificar con un fragmento de voz de prueba y sin red.
- [ ] 8.2 Implementar palabras con inicio/fin monótonos, confianza y detección/forzado de idioma; verificar con un audio en español y otro en inglés.
- [ ] 8.3 Implementar el filtro de alucinaciones por energía/sonoridad, umbral de no-speech y repeticiones; verificar con un fragmento instrumental y otro con voz.
- [ ] 8.4 Implementar la edición del texto (corregir palabra, unir/separar frase, borrar línea) conservando tiempos; verificar que la silabificación se recalcula solo en la palabra afectada.
- [ ] 8.5 Añadir tests de integración con un fixture de voz CC0 y tests unitarios con motor simulado; verificar que no se sube audio a red.
- [ ] 8.6 Documentar idiomas soportados, tamaño de modelos y límites en `docs/transcription.md`; verificar que la app muestra la misma información.

## 9. Silabificación y alineación

- [ ] 9.1 Implementar la silabificación es/en con diccionarios embebidos y reglas de diptongos/hiatos; verificar "canción", "don't", "every" y una palabra desconocida.
- [ ] 9.2 Implementar el DP monotónico sílabas↔notas con anclas por palabra y costes del diseño; verificar con un caso sintético de correspondencia uno a uno.
- [ ] 9.3 Implementar notas de continuación `~`, agrupación de sílabas sobrantes y cortes de frase; verificar melisma, notas extra y pausas.
- [ ] 9.4 Implementar confianza por sílaba/frase y marcado de dudas; verificar umbrales con casos altos y bajos.
- [ ] 9.5 Añadir tests de propiedad: ninguna sílaba se pierde, el orden temporal es monótono y ninguna frase tiene solapes; verificar con datos generados.
- [ ] 9.6 Documentar el algoritmo, supuestos y limitaciones en `docs/alignment.md`; verificar que el ejemplo del documento produce el resultado indicado.

## 10. Editor de revisión

- [ ] 10.1 Implementar el shell de la app con vista de proyecto, estados de etapa y acciones de procesar/reintentar/cancelar; verificar con un proyecto real y uno fallido.
- [ ] 10.2 Implementar reproducción con `rodio`: play, pausa, seek, bucle y cambio de stem; verificar sincronía tras 10 seeks seguidos.
- [ ] 10.3 Implementar la pirámide de picos y el render de waveform con zoom; verificar que una canción de 6 minutos se dibuja sin bloquear la UI.
- [ ] 10.4 Implementar el piano-roll con notas, continuaciones, cortes de frase, selección y resaltado de dudas; verificar con una canción generada.
- [ ] 10.5 Implementar crear, borrar, mover, redimensionar, cambiar pitch/tipo y editar texto con las restricciones de solape; verificar cada operación con tests de UI/estado.
- [ ] 10.6 Implementar controles de `#BPM`/`#GAP`, candidatos de tempo y rechazo de valores inválidos; verificar que la rejilla se actualiza sin mover el audio.
- [ ] 10.7 Implementar undo/redo de 100 pasos, autoguardado y aviso al cerrar; verificar deshacer/rehacer y un cierre con cambios pendientes.
- [ ] 10.8 Implementar exportación desde el editor y navegación al error de validación; verificar que un texto vacío selecciona la nota correcta.
- [ ] 10.9 Documentar atajos y flujo de revisión en `docs/editor.md`; verificar el checklist manual de reproducción, edición y exportación.

## 11. Integración, rendimiento y entrega

- [ ] 11.1 Ejecutar la suite completa con `USDX_REFS_DIR` y verificar que los 132 archivos se parsean y los de una voz hacen round-trip sin errores.
- [ ] 11.2 Probar el pipeline completo con una canción en español y otra en inglés; verificar la checklist de aceptación (audio, carátula, letra, notas, frases) y registrar tiempos.
- [ ] 11.3 Medir los presupuestos de rendimiento del diseño (separación, transcripción, pitch, alineación); verificar que se cumplen o documentar la degradación aplicada.
- [ ] 11.4 Generar builds release para Windows, macOS y Linux y ejecutar una prueba de humo en cada uno; verificar arranque, detección de herramientas y apertura de proyecto.
- [ ] 11.5 Revisar `docs/licenses.md` contra el `Cargo.lock` final y los binarios incluidos; verificar que no se distribuye `ffmpeg` GPL ni pesos Demucs.
- [ ] 11.6 Escribir la guía de usuario (instalación, primera canción, revisión, exportación y copia a `songs/`) en `README.md`; verificar siguiéndola de principio a fin.
- [ ] 11.7 Copiar una canción exportada a `refs/UltraStar WorldParty/songs/` en una copia de prueba y verificar que UltraStar la muestra y la reproduce.

## Workflow follow-up

- Archivar el cambio cuando la revisión del proyecto esté satisfecha.
- Verificar el resultado archivado y que las specs principales reflejan el comportamiento implementado.

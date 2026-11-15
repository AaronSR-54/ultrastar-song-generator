# Vista previa y edición

Al terminar el pipeline, la app muestra **Canción lista**. Desde ahí, **Vista previa** abre la
comprobación de la canción: se reproduce, se navega por la letra silabeada y se corrigen las notas
antes de exportar.

```bash
cargo run -p usdx-desktop
```

## Frase actual

La franja superior muestra la letra completa de la frase que suena. Cada sílaba es un elemento
clicable:

- la sílaba activa se pinta en azul;
- las continuaciones (`~`) quedan en gris;
- la sílaba seleccionada lleva un subrayado azul;
- al pulsar una sílaba, se selecciona y el audio salta a su instante exacto.

Si la posición cae entre frases (introducción o interludio), se muestra la siguiente frase y la
primera sílaba pendiente queda resaltada.

## Editar notas

La barra de edición, bajo la frase, trabaja sobre la sílaba seleccionada y muestra su texto y su
tipo. Se puede seleccionar desde la frase o desde el lienzo de notas (clic en un bloque).

| Acción | Botón | Tecla | Efecto |
|---|---|---|---|
| No cantable | **No cantable** | `F` | Escribe `F`: la sílaba se ve pero no puntúa. Se usa para partes habladas o imposibles de afinar |
| Rap | **Rap** | `R` | Escribe `R`: se canta hablado y puntúa por tiempo |
| Normal | **Normal** | `N` | Vuelve a `:` |
| Quitar sílaba | **Borrar** | `Supr` / `Retroceso` | Elimina la nota del `.txt` |
| Deshacer / rehacer | ↶ / ↷ | `Ctrl+Z` / `Ctrl+Y` | Hasta 100 pasos |

Detalles que respeta el editor:

- **Las palabras conservan sus espacios.** En el formato, la sílaba que abre palabra lleva un
  espacio delante. Si se borra esa sílaba, la siguiente hereda la marca, así que `can` + `ción` no se
  pega a la palabra anterior.
- **Marcar no cambia la letra.** `F` y `R` conservan el texto: una nota sin texto (salvo `~`) impide
  exportar.
- **El cursor no salta.** Al borrar, la selección queda en la sílaba que ocupó el hueco, para
  encadenar borrados.
- Si una frase se queda sin notas, desaparece del documento.
- Los cambios se guardan solos **2 segundos** después de la última pulsación, y **Guardar** lo hace
  al instante. Exportar también guarda.
- En el lienzo, las notas `F` se dibujan en contorno (no puntúan) y las `R`/doradas llevan una barra
  ámbar bajo el bloque.

## Ventana de notas silabeadas

La ventana sigue la reproducción con una vista de 9 segundos:

- cada bloque es una nota/sílaba; el ancho representa su duración;
- la altura representa el pitch, con una rejilla horizontal de referencia;
- la **frase actual** se ilumina y la **nota activa** se rellena en azul;
- las frases anteriores quedan atenuadas y las futuras en contorno;
- las líneas verticales marcan el inicio de cada frase;
- la regla inferior marca los segundos de la vista;
- hacer clic en un bloque lo selecciona; arrastrar hace *seek*.

Para que la melodía no quede como una línea fina, el rango de pitch se recorta al percentil 10–90 y
se limita a 14 semitonos; los valores extremos se fijan al borde.

## Transporte y velocidad

| Control | Efecto |
|---|---|
| Play / pausa | Reproduce o detiene el audio sin perder la posición |
| Detener | Pausa y vuelve a 0:00 |
| Al principio | Salta a 0:00 sin cambiar el estado de reproducción |
| Velocidad | `0,5×`, `0,75×`, `1×` o `1,25×` |

El reloj avanza a la velocidad elegida y la barra de reproducción se actualiza en el mismo frame.
`rodio` cambia la velocidad remuestreando: al ralentizar, el tono baja en la misma proporción. Si no
hay dispositivo de audio, el reloj sigue funcionando en silencio y la vista previa es igualmente
navegable.

### Atajos

| Tecla | Acción |
|---|---|
| `Espacio` | Play / pausa |
| `←` / `→` | Retroceder / avanzar 5 s |
| `Inicio` | Volver al principio |
| `F` / `R` / `N` | No cantable / rap / normal |
| `Supr` | Quitar la sílaba seleccionada |
| `Ctrl+Z` / `Ctrl+Y` | Deshacer / rehacer |
| `Esc` | Volver a la pantalla anterior |

Los atajos se ignoran mientras un campo de texto tiene el foco.

## Barra de ventana

**Ajustes → Barra de ventana** ofrece tres presentaciones:

- **Sistema:** barra nativa. En Windows 11 se tiñe con la paleta de la app usando DWM (fondo,
  texto y borde); en el resto de sistemas se respeta el aspecto del sistema.
- **Integrada:** barra de 38 px dibujada por la app: marca y controles de minimizar, maximizar y
  cerrar. Se arrastra desde cualquier punto libre y el doble clic maximiza.
- **Mínima:** barra de 28 px con la marca abreviada y los mismos controles.

La preferencia se guarda en `settings.json` y se aplica en el siguiente arranque; el cambio de modo
también se aplica en caliente sin reiniciar.

## Exportación

**Exportar** valida el documento con `usdx-format` y vuelve a escribir la carpeta:

- si hay un error (por ejemplo, una nota sin texto), no escribe nada y lo explica;
- si solo hay avisos, exporta y los muestra;
- si todo es válido, genera `output/<Artista> - <Título>/` con el `.txt`, el audio, la carátula y el
  vídeo si el proyecto lo tiene.

El `.txt` solo lleva `#VIDEO:video.mp4` cuando ese archivo está realmente en la carpeta. Así UltraStar
nunca busca un vídeo ausente: o está y se reproduce, o no hay etiqueta.

### Añadir vídeo a una canción ya procesada

Las canciones creadas antes de que el vídeo fuera parte del proceso no lo tienen. En **Canción lista**
aparece **Añadir vídeo** cuando falta:

1. descarga el vídeo del enlace original en segundo plano (se puede cancelar);
2. lo guarda en `source/video.mp4`;
3. vuelve a exportar y copia la carpeta al destino, ya con `#VIDEO:video.mp4`.

Si la descarga falla, la canción sigue lista sin vídeo y se puede reintentar.

## Límites conocidos

- Una sola voz; duetos y multipista quedan fuera del MVP.
- El tempo es constante: no hay mapa de tempo.
- Se editan el tipo y la presencia de cada nota; mover, redimensionar y afinar existen en el motor
  (`EditorState`) pero aún no tienen gesto en la interfaz.
- Cambiar de velocidad altera el pitch porque el motor remuestrea el audio.

## Checklist manual

1. Procesar una canción y abrir **Vista previa**.
2. Comprobar que la primera sílaba aparece resaltada y que la ventana de notas está centrada en ella.
3. Pulsar play: la barra avanza, la frase y la sílaba activa cambian.
4. Cambiar a `0,5×` y comprobar que el reloj avanza a la mitad de velocidad.
5. Arrastrar la barra de reproducción y verificar que la vista de notas salta al instante.
6. Pulsar una sílaba de la frase actual: el audio salta a esa sílaba y queda seleccionada.
7. Pulsar `F`: la sílaba pasa a no cantable y el bloque se dibuja en contorno. `N` la devuelve.
8. Pulsar `Supr`: desaparece la sílaba; la palabra conserva su espacio y `Ctrl+Z` la recupera.
9. Esperar 2 segundos y comprobar que el indicador pasa a **Guardado**.
10. Pulsar **Detener** y comprobar que vuelve a 0:00 y se pausa.
11. Cambiar la barra de ventana entre **Sistema**, **Integrada** y **Mínima** sin reiniciar.
12. Pulsar **Exportar** y comprobar que la carpeta de salida se reescribe y que la nota `F` aparece
    en el `.txt`.
13. Probar `Espacio`, `←`, `→`, `Inicio` y `Esc`.
14. Con una canción procesada antes de este cambio, pulsar **Añadir vídeo** en **Canción lista** y
    comprobar que el `.txt` gana `#VIDEO:video.mp4` y que UltraStar muestra el vídeo.

# Formato UltraStar soportado

El crate `usdx-format` lee y escribe el archivo `.txt` de UltraStar Deluxe. El soporte se validó contra las 132 canciones de `refs/UltraStar WorldParty/songs/`.

## Cabeceras

Las obligatorias para exportar son:

| Cabecera | Significado |
|---|---|
| `#TITLE` | Título de la canción |
| `#ARTIST` | Artista |
| `#MP3` | Archivo de audio relativo a la carpeta |
| `#BPM` | Tempo de UltraStar: **cuatro veces** el tempo musical |
| `#GAP` | Milisegundos que se suman a todos los tiempos |

Opcionales soportadas: `#LANGUAGE`, `#GENRE`, `#YEAR`, `#CREATOR`, `#COVER`, `#VIDEO`, `#VIDEOGAP`, `#BACKGROUND`, `#EDITION`, `#MEDLEYSTARTBEAT`, `#MEDLEYENDBEAT`.

Las cabeceras desconocidas se conservan en `SongHeader::extra` y se vuelven a escribir, para no perder información al leer un archivo ajeno. Al leer se emite un aviso por cada cabecera desconocida.

## Notas

Cada nota ocupa una línea:

```
<tipo> <beat_inicio> <duración_beats> <pitch> <texto>
```

| Tipo | Significado |
|---|---|
| `:` | Nota normal |
| `*` | Nota dorada |
| `F` | Freestyle (no puntúa) |
| `R` | Rap |
| `G` | Rap dorado |
| `-` | Corte de frase: `- <beat>` o `- <beat> <beat_fin>` |
| `E` | Fin de archivo |

- `beat_inicio` y `duración_beats` son enteros no negativos; la duración mínima es 1.
- El texto `~` indica una continuación de la sílaba anterior. No se repite la letra.
- Un espacio inicial adicional antes del texto marca el comienzo de una palabra. La primera nota de una frase no lo lleva.
- El pitch es un entero de semitonos respecto a MIDI 60: `0` = C4, `-2` = A#3, `9` = A4.
- Las notas de una misma voz deben estar en orden cronológico y no solaparse.

## Tiempo, BPM y GAP

`#BPM` no es el BPM musical: es cuatro veces ese valor. La duración de un beat es:

```
beat_ms = 60000 / (BPM * 4)
t_ms    = GAP + beat * beat_ms
```

Ejemplo de las referencias: `#BPM:313,34`, `#GAP:81090` y último beat `5723` dan unos 355 s, coherente con la duración del MP3 (366,9 s).

Se aceptan coma y punto decimal al leer (`313,34` y `313.34`). Al escribir se usa coma, que es lo habitual en las canciones de referencia, y la representación más corta que conserva el valor.

## Ejemplo

```txt
#TITLE:My Song
#ARTIST:Artist
#LANGUAGE:Spanish
#MP3:audio.mp3
#COVER:cover.jpg
#BPM:120
#GAP:1000
: 0 2 0 Hel
: 2 2 0 lo
: 4 2 0  world
: 6 4 2 ~
- 10
* 12 2 4  again
E
```

Este ejemplo se parsea en el test `documented_example.rs`; si cambias el bloque, el test debe seguir pasando.

## Codificación y compatibilidad

- Se escribe UTF-8 sin BOM y con saltos de línea LF.
- Al leer se aceptan BOM, CRLF/LF y texto no UTF-8 (se convierte con reemplazo y se avisa).
- Los nombres de archivo de `#MP3`, `#COVER` y `#VIDEO` son relativos a la carpeta de la canción.
- Al exportar, `#VIDEO` solo se escribe si `video.mp4` está realmente en la carpeta; una etiqueta
  huérfana se elimina.
- Se aceptan etiquetas en mayúsculas o minúsculas.

## Duetos

El parser reconoce las secciones `P1` y `P2`, marca `SongDoc::is_duet` y conserva las notas de P1; las de P2 se ignoran con un aviso. El MVP no genera duetos y el writer rechaza un documento marcado como dueto. Los archivos con `#DUETSINGERP1`/`#DUETSINGERP2` pero sin secciones `P1`/`P2` se tratan como una sola voz, que es como están escritos en la colección de referencia.

## Validación

Antes de exportar se ejecuta `validate(&doc)`:

- Cabeceras obligatorias presentes y BPM positivo.
- Beats y duraciones válidos, pitch dentro de `-60..=67` (MIDI 0..127).
- Sin solapes dentro de una voz, notas en orden.
- Texto no vacío salvo continuaciones.
- Cortes de frase ordenados y posteriores al final de la última nota.

Cada error incluye el índice de frase y de nota cuando corresponde, para que el editor pueda seleccionar la nota culpable.

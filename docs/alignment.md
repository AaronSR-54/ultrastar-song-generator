# Silabificación y alineación

Convierte la letra transcrita (con tiempos por palabra) y las notas detectadas (en beats) en la secuencia cantable que se exporta a UltraStar: cada sílaba queda en una nota, con continuaciones `~` para el melisma y cortes de frase en las pausas.

Todo el proceso es local y determinista: la misma entrada y las mismas opciones producen el mismo `SongDoc`.

## Silabificación

Se usan diccionarios Knuth-Liang embebidos en el binario (`hyphenation` 0.8.4):

| Idioma | Diccionario | Ejemplo |
|---|---|---|
| `es` | `Language::Spanish` | `can` + `ción` |
| `en` | `Language::EnglishUS` | `ev` + `ery` |

La división es ortográfica, no fonética. Sobre el resultado español se aplica una capa de reglas:
- **Diptongo:** se unen dos sílabas cuando la frontera es vocal débil + fuerte (`iu`, `ue`), fuerte + débil (`ai`, `ea`) o débil + débil (`iu`, `ui`). Por eso `ci|u|dad` pasa a `ciu|dad`.
- **Hiato:** dos vocales fuertes (`a`, `e`, `o`, también acentuadas) no se unen: `ca|os`.
- **Vocal débil acentuada:** `í` y `ú` rompen el diptongo; `pa|í|s` se mantiene y no se convierte en `paí|s`.
- **Contracciones y compuestos:** una palabra con apóstrofo o guion (`don't`, `hard-nosed`) se mantiene como una sola unidad silábica para no generar sílabas vacías ni alterar la letra.

En inglés los patrones no dividen algunas palabras cortas de uso frecuente (`every` queda entera). Para esos casos hay una lista corta de excepciones embebida (`every → ev|ery`, `never`, `ever`, `even`, `over`, `seven`, `heaven`, `forever`, `another`, `because`); el resto de palabras usa los patrones.

Cuando el idioma no tiene diccionario (o el diccionario no está disponible), se aplica `fallback_split` (un grupo por tirada de vocales) y la palabra se marca como desconocida. Las sílabas de una palabra desconocida reciben menos confianza y acaban resaltadas en el editor.

`DictionarySyllabifier` se carga una sola vez por proceso (`dictionary_syllabifier()`), porque deserializar los patrones no es gratis y los datos son inmutables.

## Anclas por palabra

Whisper da el intervalo de cada palabra, no de cada sílaba. El aligner reparte ese intervalo entre sus sílabas de forma proporcional a su longitud en caracteres:

```
sílaba_k.start = palabra.start + (palabra.end - palabra.start) * consumido / total
sílaba_k.end   = palabra.start + (palabra.end - palabra.start) * (consumido + peso_k) / total
```

Los segundos se convierten a beats con `#BPM` y `#GAP` mediante `usdx_format::ms_to_beat`. La confianza de la sílaba parte de la confianza de la palabra; si la palabra no está en el diccionario se multiplica por 0.65.

Cuando la letra viene de un LRC con tiempos por línea, `Transcript::word_timed` es `false`: solo los bordes de línea son fiables y las palabras se reparten por longitud dentro de la línea.

## Desfase entre versiones

LRCLIB suele devolver un registro sincronizado de otra subida de la misma canción: intro distinta, interludio más largo, duración parecida. Aplicar esos tiempos tal cual desplaza toda la letra.

Antes de alinear, si la transcripción no tiene tiempos por palabra, la etapa `Align` mide los tramos vocales del stem con `EnergyTrack::voiced_segments` y compara el inicio de cada estrofa (primera línea tras un silencio en el LRC) con el inicio del tramo vocal más cercano. Se queda con la mediana, descarta los valores a más de 1 s y aplica el desplazamiento solo si queda entre 0.3 s y 8 s. Con eso la letra entra en la canción aunque el registro venga de otra versión; sin desfase el ajuste es 0 y no cambia nada.

## Reparto de notas entre líneas

Cada línea se queda con las notas que empiezan entre su primera sílaba y la primera sílaba de la línea siguiente. Los inicios de línea son la parte más fiable del LRC; los tiempos por palabra de dentro de la línea son una estimación y no deben decidir a qué línea pertenece una nota. Las notas anteriores a la primera línea se asignan a la primera línea y las sobrantes del final a la última.

## DP monotónico

Sea `S` el número de sílabas de una línea y `N` las notas de esa línea. El DP calcula el coste mínimo de consumir todas las sílabas y todas las notas:

| Transición | Efecto | Coste |
|---|---|---|
| `Syllables { start, count: 1 }` | una sílaba → una nota | distancia temporal + duración + continuidad de pitch |
| `Syllables { start, count: k>1 }` | `k` sílabas repartidas en una nota | `0.9 + 0.35·(k-1)` + coste temporal de la primera sílaba |
| `NoteOnly` | la nota se convierte en `~` | `0.5` |

- **Distancia temporal:** `min(|beat_nota − beat_sílaba| / tolerancia, 4)`, con `tolerancia = time_tolerance_s · BPM · 4 / 60`. La tolerancia se define en segundos (0.3 s por defecto) y no en beats: un beat puede durar 40 ms (BPM alto) o 250 ms (BPM bajo), y una tolerancia fija en beats sería mucho más estricta en canciones rápidas.
- **Duración:** diferencia entre la longitud de la nota y la duración esperada del ancla, normalizada y multiplicada por 0.4.
- **Pitch:** salto de más de una octava respecto a la nota anterior, como máximo 0.1. Solo desempata.
- **Saltar nota** es más barato que saltar sílaba, así que una nota instrumental dentro de una palabra se convierte en continuación en lugar de robarle el texto a la sílaba siguiente.

El DP es monotónico por construcción: solo avanza índices, nunca reordena. Complejidad `O(S·N·K)` con `K = 4` agrupaciones como máximo.

## Continuaciones, reparto y cortes

- **Melisma:** si hay más notas que sílabas, las sobrantes llevan `text = "~"` y la confianza de la nota anterior × 0.9. La letra no se duplica.
- **Varias sílabas por nota:** el detector de pitch funde en una sola nota todas las sílabas cantadas al mismo tono. UltraStar necesita una nota por sílaba, así que la nota se divide en tantas partes como sílabas, con al menos un beat cada una y el mismo pitch. La letra no se pierde y no se marca para revisión solo por el reparto: el estado depende de la confianza temporal.
- **Línea sin notas:** si una línea no tiene ninguna nota candidata, se crea una nota sintética de un beat con pitch 0 y se marca para revisión. Así no se pierde letra.
- **Línea que no cabe:** si el DP no encuentra camino (demasiadas sílabas para las notas disponibles, por ejemplo porque la letra empieza antes que el audio), se reparten todas las sílabas en las notas disponibles y se marcan. Nunca se descarta una línea.
- **Corte de frase:** se inserta cuando cambia la línea de la transcripción o cuando el silencio entre dos notas supera `phrase_gap_s` (0.8 s por defecto). El corte se escribe como `- <fin_frase> <inicio_siguiente>`; si no hay hueco, `- <fin_frase>`.

## Confianza

```
confianza = confianza_palabra · (0.35 + 0.65·exp(−(distancia/2)²))
```

y después:

- palabra fuera de diccionario → ×0.75;
- nota agrupada → ×0.7.

Se marca `needs_review` si la confianza queda por debajo de `min_confidence` (0.55) o si la palabra era desconocida. La confianza de la frase es la media de sus notas con letra; `Alignment::is_ready()` solo devuelve `true` cuando se asignaron todas las sílabas y no hay notas para revisar.

## Ejemplo del documento

Entrada: `#BPM 120`, `#GAP 0`, una palabra `canción` entre 0.0 s y 0.5 s con confianza 0.9, dividida en `can` y `ción`, y dos notas de dos beats en los beats 0 y 2.

Salida (comprobada por el test `documented_example_matches_the_code`):

```
can start=0 length=2 pitch=0
ción start=2 length=2 pitch=0
```

A 120 BPM en el archivo cada beat dura 125 ms, así que las dos notas cubren exactamente el intervalo de la palabra.

## Supuestos y límites

- Una sola voz. En duetos el MVP alinea la mezcla y el usuario corrige.
- La alineación es temporal, no acústica: no se ejecuta un modelo de alineación forzada. Las palabras que Whisper coloca mal producen notas dudosas, no notas imposibles.
- Con letras LRC sincronizadas por línea, los tiempos internos de cada sílaba son una estimación proporcional: el desfase global se corrige con el stem vocal, pero la posición exacta de cada sílaba dentro de la línea no se conoce. Las notas detectadas sí son reales y la letra se reparte sobre ellas.
- La silabificación inglesa es ortográfica; palabras como `every` → `ev-ery` pueden no coincidir con la sílaba cantada. El editor permite corregir la letra.
- Las notas sintéticas (línea sin notas) son un recurso de último momento y siempre se marcan para revisión.
- No se admite mapa de tempo: si el BPM estimado deriva, la alineación se degrada hacia el final de la canción (ver `docs/pitch-and-tempo.md`).

## Verificación

```bash
cargo test -p usdx-ml --lib syllabify
cargo test -p usdx-ml --lib alignment
```

Los tests cubren:

- `canción`, `don't`, `every` y una palabra desconocida;
- correspondencia uno a uno, melisma, sílabas sobrantes, líneas que no caben y pausas;
- reparto de una nota detectada entre varias sílabas sin perder texto;
- corrección del desfase global con tramos vocales, y que las letras con tiempos por palabra no se desplacen;
- umbrales de confianza altos y bajos;
- propiedades con datos generados: no se pierde ninguna sílaba, los inicios son monótonos, no hay solapes dentro de una frase y el texto no cambia;
- escritura y relectura del `.txt` generado.

# Spec Delta

## Purpose

Lee, valida y escribe el archivo `.txt` de UltraStar Deluxe, incluyendo cabeceras, notas, continuaciones, cortes de frase y la relación entre beats, BPM y GAP.

## ADDED Requirements

### Requirement: Escritura de cabeceras obligatorias

El sistema SHALL escribir como mínimo `#TITLE`, `#ARTIST`, `#BPM` y `#MP3`, y SHALL incluir `#COVER`, `#LANGUAGE`, `#GENRE`, `#YEAR`, `#CREATOR` y `#VIDEO` cuando existan valores.

#### Scenario: Exportación completa

- **WHEN** el proyecto tiene título, artista, audio, carátula e idioma
- **THEN** el `.txt` contiene esas cabeceras con los nombres de archivo relativos a la carpeta de la canción

#### Scenario: Artista pendiente

- **WHEN** el artista está vacío
- **THEN** el sistema bloquea la exportación e indica el campo que falta

### Requirement: Escritura de notas

El sistema SHALL escribir cada nota como tipo, beat de inicio, duración en beats, pitch y texto, usando `:` para notas normales, `*` para notas doradas, `-` para cortes de frase y `E` como marca final.

#### Scenario: Nota normal

- **WHEN** una sílaba empieza en el beat 12, dura 4 beats y suena en C4
- **THEN** el sistema escribe una línea equivalente a `: 12 4 0 texto`

#### Scenario: Continuación

- **WHEN** una nota continúa la sílaba anterior sin texto nuevo
- **THEN** el sistema escribe `~` como texto y no repite la letra

#### Scenario: Fin de archivo

- **WHEN** se termina de escribir la última frase
- **THEN** el archivo incluye una marca final `E`

### Requirement: Conversión entre tiempo y beats

El sistema SHALL calcular el tiempo de una nota como `GAP + beat * 60000 / (BPM * 4)` milisegundos y SHALL escribir `#BPM` como cuatro veces el tempo musical. Al leer, SHALL aplicar la misma fórmula de forma inversa.

#### Scenario: Ida y vuelta temporal

- **WHEN** se exporta una nota y se vuelve a leer el archivo generado
- **THEN** el beat de inicio, la duración y el pitch coinciden con los exportados

#### Scenario: Aceptar coma y punto decimal

- **WHEN** un archivo de referencia escribe `#BPM:313,34` y otro `#BPM:313.34`
- **THEN** el sistema lee ambos como el mismo valor

### Requirement: Restricciones de notas válidas

El sistema SHALL validar que los beats de inicio son enteros no negativos, que la duración es al menos 1, que el pitch es un entero dentro del rango admitido y que las notas de una voz no se solapan.

#### Scenario: Duración cero

- **WHEN** una nota queda con duración 0 tras la cuantización
- **THEN** el sistema la ajusta a 1 beat antes de escribir

#### Scenario: Texto vacío

- **WHEN** una nota no es continuación y su texto está vacío
- **THEN** el sistema bloquea la exportación de esa nota y la señala en el editor

### Requirement: Lectura de archivos existentes

El sistema SHALL leer archivos UltraStar de una sola voz, incluyendo cabeceras, notas normales y doradas, continuaciones y cortes de frase, y SHALL informar de líneas mal formadas sin abortar el resto de la lectura.

#### Scenario: Archivo de referencia

- **WHEN** se abre un `.txt` de la colección de referencia
- **THEN** el sistema obtiene sus cabeceras y notas y puede volver a escribir un archivo semánticamente equivalente

#### Scenario: Dueto

- **WHEN** el archivo contiene secciones `P1` y `P2`
- **THEN** el sistema lo detecta como dueto y lo informa como no soportado para generación en el MVP

### Requirement: Carpeta de canción exportable

El sistema SHALL exportar una carpeta que contenga el `.txt`, el audio, la carátula y, si existe, el vídeo, con los nombres exactos referenciados en las cabeceras.

#### Scenario: Exportación a UltraStar

- **WHEN** el usuario exporta una canción
- **THEN** la carpeta resultante puede copiarse a `songs/` de UltraStar WorldParty y la canción aparece con audio y carátula

#### Scenario: Destino ocupado

- **WHEN** ya existe una carpeta con el mismo nombre en el destino
- **THEN** el sistema pide confirmación antes de sobrescribir y ofrece elegir otro nombre

### Requirement: Codificación y compatibilidad

El sistema SHALL escribir los archivos en UTF-8 y SHALL usar saltos de línea compatibles con UltraStar Deluxe en Windows, macOS y Linux.

#### Scenario: Acentos y eñes

- **WHEN** la letra contiene caracteres como `á`, `ñ` o `ü`
- **THEN** el archivo exportado los conserva y UltraStar los muestra correctamente

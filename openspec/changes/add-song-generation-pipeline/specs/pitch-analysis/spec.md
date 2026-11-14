# Spec Delta

## Purpose

Convierte la pista vocal en una secuencia de notas con altura, inicio y duración, y calcula la rejilla de tempo necesaria para expresarlas en el formato UltraStar.

## ADDED Requirements

### Requirement: Seguimiento de altura fundamental

El sistema SHALL estimar la frecuencia fundamental de la voz por tramas cortas, con marca de sonoridad y confianza, y SHALL descartar tramas no cantadas o sin energía suficiente.

#### Scenario: Nota sostenida

- **WHEN** la voz mantiene una altura estable durante al menos la duración mínima configurada
- **THEN** el sistema produce una nota con esa altura y una confianza alta

#### Scenario: Silencio o ruido

- **WHEN** una trama no supera el umbral de sonoridad
- **THEN** el sistema no genera pitch para esa trama y no la convierte en nota

### Requirement: Segmentación en notas

El sistema SHALL agrupar tramas contiguas de altura similar en notas, separando cuando la altura cambia de semitono de forma sostenida o cuando hay una pausa, y SHALL aplicar una duración mínima para evitar notas espurias.

#### Scenario: Cambio de nota

- **WHEN** la altura se mantiene al menos un semitono distinta durante el tiempo mínimo
- **THEN** el sistema cierra la nota anterior y abre una nueva en el instante del cambio

#### Scenario: Temblor o vibrato

- **WHEN** la altura oscila alrededor de una misma nota sin establecerse en otra
- **THEN** el sistema mantiene una sola nota y no fragmenta el vibrato

### Requirement: Mapeo al pitch de UltraStar

El sistema SHALL expresar cada nota como un entero de semitonos donde 0 corresponde a MIDI 60 (C4), redondeando la altura media de la nota al semitono más cercano.

#### Scenario: Conversión de frecuencia

- **WHEN** una nota tiene una altura media de 440 Hz
- **THEN** el sistema la exporta como pitch 9 y conserva la frecuencia original en los artefactos de análisis

#### Scenario: Nota fuera de rango

- **WHEN** la altura supera el rango representable del formato
- **THEN** el sistema recorta al límite y registra una advertencia con la nota afectada

### Requirement: Estimación de tempo y offset

El sistema SHALL estimar un tempo musical y un offset inicial, SHALL presentar alternativas cuando la estimación sea ambigua y SHALL permitir fijar manualmente `#BPM` y `#GAP`. El valor de `#BPM` exportado SHALL ser cuatro veces el tempo musical estimado.

#### Scenario: Tempo claro

- **WHEN** la canción tiene pulso regular y el análisis obtiene una estimación con confianza suficiente
- **THEN** el sistema propone `#BPM` y `#GAP` y los muestra en el editor

#### Scenario: Tempo ambiguo

- **WHEN** el análisis detecta candidatos a la mitad o al doble del tempo principal
- **THEN** el sistema muestra los candidatos y no elige uno sin confirmación del usuario

#### Scenario: Ajuste manual

- **WHEN** el usuario modifica `#BPM` o `#GAP`
- **THEN** el sistema recalcula la posición y duración en beats de las notas existentes sin repetir el análisis de audio

### Requirement: Cuantización a la rejilla de beats

El sistema SHALL convertir inicio y fin de cada nota a beats enteros según `#GAP` y `#BPM`, con longitud mínima de un beat, sin solapamientos dentro de una misma voz.

#### Scenario: Nota más corta que un beat

- **WHEN** una nota dura menos que un beat en la rejilla activa
- **THEN** el sistema la ajusta a un beat y evita que invada la nota siguiente

#### Scenario: Notas solapadas tras cuantizar

- **WHEN** dos notas quedan en el mismo beat de inicio tras la cuantización
- **THEN** el sistema fusiona o desplaza la segunda según la confianza y registra el ajuste

### Requirement: Reproducibilidad del análisis

El sistema SHALL guardar la configuración y los resultados del análisis para que abrir el proyecto produzca las mismas notas sin volver a ejecutar la inferencia.

#### Scenario: Reabrir proyecto

- **WHEN** el usuario cierra y vuelve a abrir un proyecto ya analizado
- **THEN** las notas, `#BPM`, `#GAP` y puntuaciones de confianza coinciden con los guardados

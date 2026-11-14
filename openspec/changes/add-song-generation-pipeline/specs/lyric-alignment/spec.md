# Spec Delta

## Purpose

Divide la letra en sílabas y asigna cada sílaba a una nota y a un instante concretos, produciendo la secuencia cantable que se exportará a UltraStar.

## ADDED Requirements

### Requirement: Silabificación por idioma

El sistema SHALL dividir cada palabra en sílabas según el idioma activo, respetando acentos, diptongos, triptongos y contracciones habituales del español y del inglés.

#### Scenario: Palabra española con diptongo

- **WHEN** se silabifica una palabra como "canción"
- **THEN** el sistema produce las sílabas "can" y "ción"

#### Scenario: Contracción inglesa

- **WHEN** se silabifica una contracción como "don't"
- **THEN** el sistema la mantiene como una unidad silábica y no genera una sílaba vacía

#### Scenario: Palabra desconocida

- **WHEN** el diccionario no reconoce una palabra
- **THEN** el sistema aplica una división de respaldo y la marca para revisión

### Requirement: Asignación de sílabas a notas

El sistema SHALL asignar en orden cada sílaba a una nota, usando las marcas temporales de las palabras como anclas y los onsets de las notas como candidatos, sin reordenar ni perder sílabas.

#### Scenario: Correspondencia uno a uno

- **WHEN** una palabra tiene dos sílabas y su intervalo contiene dos notas
- **THEN** cada sílaba se asigna a una nota y los tiempos quedan dentro del intervalo de la palabra

#### Scenario: Sílaba sin nota disponible

- **WHEN** una palabra tiene más sílabas que notas en su intervalo
- **THEN** el sistema agrupa las sílabas sobrantes en la nota más cercana y marca la asignación como dudosa

### Requirement: Notas de continuación

El sistema SHALL representar con notas de continuación las notas adicionales que pertenecen a una sílaba ya cantada, de modo que la letra no se duplique.

#### Scenario: Melisma

- **WHEN** una sílaba se mantiene mientras la altura cambia en dos notas
- **THEN** la primera nota lleva el texto y la segunda se exporta como continuación

#### Scenario: Nota extra al final de una palabra

- **WHEN** hay más notas que sílabas en el intervalo de una palabra
- **THEN** las notas sobrantes se convierten en continuaciones de la última sílaba

### Requirement: Frases y silencios

El sistema SHALL insertar cortes de frase en las pausas de voz y en los cambios de línea, y SHALL garantizar que ninguna frase contenga notas solapadas.

#### Scenario: Pausa entre frases

- **WHEN** hay un silencio superior al umbral entre dos grupos de notas
- **THEN** el sistema inserta un corte de frase en el beat correspondiente

#### Scenario: Cambio de línea

- **WHEN** la letra cambia de línea
- **THEN** el sistema coloca un corte de frase antes de la primera nota de la nueva línea

### Requirement: Puntuación de confianza y avisos

El sistema SHALL calcular una confianza por sílaba y por frase y SHALL señalar las asignaciones que requieren revisión humana.

#### Scenario: Asignación dudosa

- **WHEN** el tiempo de una sílaba está lejos de cualquier onset o su palabra tiene baja confianza
- **THEN** la sílaba se marca como dudosa y aparece resaltada en el editor

#### Scenario: Canción sin avisos

- **WHEN** todas las sílabas superan el umbral de confianza
- **THEN** el sistema indica que la alineación está lista para exportar

### Requirement: Corrección manual de la alineación

El sistema SHALL permitir mover una sílaba a otra nota, fusionar o separar sílabas, convertir una nota en continuación y ajustar su inicio, duración o altura.

#### Scenario: Mover sílaba

- **WHEN** el usuario arrastra una sílaba a otra nota
- **THEN** la nota de origen se elimina o queda vacía y la de destino recibe la sílaba sin romper el orden temporal

#### Scenario: Separar sílabas agrupadas

- **WHEN** el usuario separa dos sílabas que comparten una nota
- **THEN** el sistema crea una nota adicional con la altura y tiempo más plausibles y marca el resultado para revisión

# Spec Delta

## Purpose

Extrae la letra cantada de la pista vocal con marcas temporales por palabra y detección de idioma, lista para alinearse con las notas.

## ADDED Requirements

### Requirement: Transcripción local

El sistema SHALL transcribir la pista vocal con un motor de reconocimiento que se ejecute en el equipo, sin subir audio a servicios externos.

#### Scenario: Canción en español

- **WHEN** el usuario analiza una canción cantada en español
- **THEN** el sistema produce líneas de texto en español con marcas temporales por palabra

#### Scenario: Canción en inglés

- **WHEN** el usuario analiza una canción cantada en inglés
- **THEN** el sistema produce líneas de texto en inglés con marcas temporales por palabra

### Requirement: Detección y selección de idioma

El sistema SHALL detectar el idioma de la letra y SHALL permitir forzarlo antes o después de transcribir.

#### Scenario: Idioma detectado

- **WHEN** la transcripción termina con confianza de idioma suficiente
- **THEN** el sistema muestra el idioma detectado y lo usa para silabificar

#### Scenario: Idioma forzado

- **WHEN** el usuario fija manualmente el idioma y vuelve a transcribir
- **THEN** el sistema usa ese idioma para el reconocimiento y actualiza la silabificación

### Requirement: Marcas temporales por palabra

El sistema SHALL asignar a cada palabra un inicio y un fin dentro del audio, monótonos y no solapados, y SHALL exponer su confianza.

#### Scenario: Palabras con tiempos

- **WHEN** una frase se transcribe correctamente
- **THEN** cada palabra tiene inicio y fin en segundos y el fin no supera el inicio de la palabra siguiente

#### Scenario: Palabra sin tiempo fiable

- **WHEN** una palabra obtiene confianza por debajo del umbral
- **THEN** el sistema la marca como dudosa y la resalta en el editor

### Requirement: Supresión de alucinaciones e instrumental

El sistema SHALL descartar texto generado en regiones sin voz, usando la energía y sonoridad de la pista vocal como referencia.

#### Scenario: Introducción instrumental

- **WHEN** los primeros 30 segundos no contienen voz
- **THEN** el sistema no genera letra para esa región

#### Scenario: Texto repetido sin voz

- **WHEN** el motor devuelve frases durante un pasaje instrumental
- **THEN** el sistema elimina esas frases y registra cuántas descartó

### Requirement: Edición del texto transcrito

El sistema SHALL permitir corregir palabras, unir o separar frases y eliminar líneas antes de la alineación.

#### Scenario: Corrección de una palabra

- **WHEN** el usuario corrige una palabra mal transcrita
- **THEN** el sistema conserva los tiempos de la frase y recalcula la silabificación de esa palabra

#### Scenario: Eliminar línea

- **WHEN** el usuario elimina una línea de letra
- **THEN** sus palabras dejan de participar en la alineación y la línea desaparece de la exportación

### Requirement: Progreso y cancelación

El sistema SHALL mostrar el avance de la transcripción y SHALL permitir cancelarla sin dejar un resultado parcial como válido.

#### Scenario: Cancelación

- **WHEN** el usuario cancela durante la transcripción
- **THEN** el sistema descarta el resultado parcial y mantiene la etapa pendiente

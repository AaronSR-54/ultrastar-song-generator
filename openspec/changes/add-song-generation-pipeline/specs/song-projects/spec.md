# Spec Delta

## Purpose

Gestiona el ciclo de vida de cada canción en preparación: creación, estructura de trabajo, estado de las etapas, reanudación, invalidación y control del procesamiento.

## ADDED Requirements

### Requirement: Creación de un proyecto de canción

El sistema SHALL crear un proyecto a partir de una URL de vídeo válida y un directorio de destino, con un identificador estable y un nombre de archivo seguro. El proyecto SHALL quedar guardado en disco antes de iniciar la descarga.

#### Scenario: Crear proyecto nuevo

- **WHEN** el usuario introduce una URL válida y elige un directorio de destino
- **THEN** el sistema crea una carpeta de proyecto con un identificador único y un archivo de proyecto persistente
- **AND** muestra el proyecto en estado inicial con todas las etapas pendientes

#### Scenario: URL inválida

- **WHEN** el usuario introduce un texto que no es una URL http o https compatible
- **THEN** el sistema rechaza la creación y explica el motivo sin crear carpetas

### Requirement: Separación entre original, artefactos intermedios y salida

El proyecto SHALL mantener separados los medios descargados, los artefactos intermedios de análisis y la carpeta final exportable, de modo que reexportar no vuelva a descargar ni a analizar.

#### Scenario: Reexportar sin reprocesar

- **WHEN** el usuario cambia solo el título o la carátula y solicita exportar de nuevo
- **THEN** el sistema reutiliza los medios y análisis existentes y solo regenera la salida

#### Scenario: Localizar artefactos

- **WHEN** el usuario abre un proyecto existente
- **THEN** el sistema encuentra y carga los medios, stems, transcripción, notas y ajustes guardados

### Requirement: Estado y reanudación por etapa

El sistema SHALL registrar el estado de cada etapa como pendiente, en curso, completada, fallida u omitida, y SHALL reanudar un proyecto fallido desde la primera etapa no completada.

#### Scenario: Reanudar tras un fallo

- **WHEN** un proyecto falla durante la separación de voz y el usuario lo vuelve a abrir
- **THEN** el sistema conserva las etapas anteriores completadas y reanuda desde la separación

#### Scenario: Etapa omitida

- **WHEN** el usuario desactiva la descarga de vídeo pero mantiene audio y carátula
- **THEN** la etapa de vídeo queda marcada como omitida y el resto del flujo continúa

### Requirement: Invalidación por cambios de entrada

El sistema SHALL invalidar las etapas dependientes cuando cambie una entrada relevante, y SHALL conservar las etapas no afectadas.

#### Scenario: Cambio de calidad de audio

- **WHEN** el usuario cambia la calidad de audio y vuelve a procesar
- **THEN** se invalidan descarga, separación, transcripción, pitch y alineación
- **AND** se conservan los metadatos y la carátula si no cambiaron

#### Scenario: Cambio de idioma de transcripción

- **WHEN** el usuario cambia el idioma forzado de la transcripción
- **THEN** se invalidan transcripción, alineación y exportación, pero no la separación ni el pitch

### Requirement: Progreso y cancelación

El sistema SHALL informar del progreso de la etapa activa y SHALL permitir cancelarla sin corromper el proyecto ni los artefactos ya completados.

#### Scenario: Cancelación durante la descarga

- **WHEN** el usuario cancela mientras se descarga el audio
- **THEN** el sistema detiene la descarga, elimina los archivos parciales y deja el proyecto reanudable

#### Scenario: Cancelación durante la separación

- **WHEN** el usuario cancela la separación de voz
- **THEN** el sistema libera los recursos de inferencia y marca la etapa como pendiente, no como fallida

### Requirement: Ejecución de una sola tarea pesada

El sistema SHALL impedir que dos etapas pesadas del mismo proyecto se ejecuten a la vez y SHALL mantener la interfaz responsive mientras procesa.

#### Scenario: Intento de doble ejecución

- **WHEN** el usuario pulsa procesar mientras ya hay una etapa en curso
- **THEN** el sistema ignora la nueva orden y muestra que ya hay un procesamiento activo

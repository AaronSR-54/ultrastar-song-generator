# Spec Delta

## Purpose

Aísla localmente la voz cantada del resto de instrumentos para que el análisis de tono y la transcripción trabajen sobre una señal limpia.

## ADDED Requirements

### Requirement: Separación local en dos stems

El sistema SHALL separar el audio en una pista de voz y una pista instrumental sin enviar audio a servicios externos y sin requerir Python en tiempo de ejecución.

#### Scenario: Separación correcta

- **WHEN** el usuario inicia la separación de un audio descargado
- **THEN** el proyecto contiene una pista `vocals` y una pista `instrumental`
- **AND** ambas tienen la misma frecuencia de muestreo, canales y duración que el audio de entrada

#### Scenario: Sin conexión tras preparar modelos

- **WHEN** los modelos ya están descargados y el equipo no tiene red
- **THEN** la separación se ejecuta igualmente y no intenta contactar con ningún servicio

### Requirement: Reconstrucción de la mezcla

El sistema SHALL guardar los stems de forma que la suma de voz e instrumental se aproxime a la mezcla original, sin recortes ni desplazamientos temporales entre pistas.

#### Scenario: Comprobación de alineación

- **WHEN** el sistema termina la separación
- **THEN** valida que voz e instrumental empiezan en la misma muestra y que su suma no se desvía de la mezcla más de lo permitido por la tolerancia configurada

### Requirement: Progreso y cancelación de la inferencia

El sistema SHALL mostrar el avance de la separación por fragmentos y SHALL cancelar liberando memoria y sin dejar stems parciales utilizables.

#### Scenario: Cancelación a mitad

- **WHEN** el usuario cancela la separación
- **THEN** el sistema detiene la inferencia, elimina los stems parciales y deja la etapa pendiente

#### Scenario: Progreso visible

- **WHEN** la separación está en curso
- **THEN** el sistema actualiza el porcentaje y el tiempo estimado restante al menos cada pocos segundos

### Requirement: Fallo de memoria o backend

El sistema SHALL detectar errores de memoria del backend de inferencia y SHALL reintentar en CPU cuando el fallo sea recuperable, informando del cambio.

#### Scenario: GPU sin memoria

- **WHEN** el backend seleccionado agota la memoria durante la separación
- **THEN** el sistema reintenta en CPU con una advertencia visible
- **AND** si CPU tampoco puede completar, marca la etapa como fallida con una causa comprensible

### Requirement: Aviso de voz no detectada

El sistema SHALL medir la energía de la pista vocal y SHALL advertir cuando no se detecte voz suficiente para generar notas fiables.

#### Scenario: Audio instrumental

- **WHEN** la pista vocal resultante está por debajo del umbral de energía durante casi toda la canción
- **THEN** el sistema muestra una advertencia y sugiere revisar la fuente o continuar solo con transcripción manual

### Requirement: Reproducción comparativa

El sistema SHALL permitir reproducir la mezcla, la voz aislada y el instrumental por separado desde el editor.

#### Scenario: Escucha de la pista vocal

- **WHEN** el usuario selecciona la pista vocal en el editor
- **THEN** la reproducción usa el stem vocal y respeta la posición y el volumen actuales

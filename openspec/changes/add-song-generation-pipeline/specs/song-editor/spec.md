# Spec Delta

## Purpose

Ofrece la pantalla de revisión donde el usuario escucha la canción, inspecciona la alineación generada y corrige texto, tiempos, pitch y frases antes de exportar.

## ADDED Requirements

### Requirement: Vista de proyecto y etapas

El sistema SHALL mostrar las etapas del proyecto con su estado, el progreso de la etapa activa y las acciones disponibles según el estado.

#### Scenario: Proyecto recién creado

- **WHEN** el usuario abre un proyecto nuevo
- **THEN** ve todas las etapas pendientes y puede iniciar el procesamiento completo o una etapa concreta

#### Scenario: Etapa fallida

- **WHEN** una etapa falla
- **THEN** la vista muestra la causa, la acción de reintentar y las etapas que dependen de ella

### Requirement: Reproducción sincronizada

El sistema SHALL reproducir el audio con play, pausa, seek y bucle, y SHALL mantener sincronizados el cursor, la waveform y el piano-roll.

#### Scenario: Seek en la waveform

- **WHEN** el usuario hace clic en un punto de la waveform
- **THEN** la reproducción salta a ese instante y el cursor y las notas visibles se actualizan

#### Scenario: Bucle de revisión

- **WHEN** el usuario define un rango de bucle sobre una frase
- **THEN** la reproducción repite ese rango hasta que lo desactive

### Requirement: Visualización de waveform y notas

El sistema SHALL mostrar la waveform de la mezcla o del stem seleccionado y un piano-roll con las notas generadas, sus continuaciones y los cortes de frase.

#### Scenario: Cambio de stem

- **WHEN** el usuario alterna entre mezcla, voz e instrumental
- **THEN** la waveform cambia al stem elegido sin perder la posición de reproducción

#### Scenario: Nota dudosa

- **WHEN** una nota tiene confianza baja
- **THEN** el piano-roll la resalta visualmente y el usuario puede ver su motivo

### Requirement: Edición de notas

El sistema SHALL permitir crear, eliminar, mover, redimensionar y cambiar el pitch o el tipo de una nota, así como editar su texto.

#### Scenario: Ajustar el inicio

- **WHEN** el usuario arrastra una nota horizontalmente
- **THEN** su beat de inicio cambia y el sistema impide que invada la nota anterior o siguiente

#### Scenario: Cambiar pitch

- **WHEN** el usuario sube una nota un semitono
- **THEN** el valor de pitch aumenta en 1 y el cambio se refleja en la exportación

#### Scenario: Añadir nota

- **WHEN** el usuario inserta una nota en un hueco de la frase
- **THEN** el sistema crea una nota con pitch y duración editables y la incluye en el orden temporal

### Requirement: Control de tempo y offset

El sistema SHALL exponer `#BPM` y `#GAP` como controles editables, SHALL reaccionar a los cambios recalculando la rejilla visual y SHALL permitir comparar candidatos de tempo.

#### Scenario: Ajuste fino del offset

- **WHEN** el usuario modifica `#GAP` en milisegundos
- **THEN** la rejilla y las notas se desplazan en la vista previa sin moverse en el tiempo real del audio

#### Scenario: Rechazo de valor inválido

- **WHEN** el usuario introduce un BPM cero o negativo
- **THEN** el sistema no aplica el valor y muestra el rango válido

### Requirement: Deshacer y rehacer

El sistema SHALL registrar las ediciones de notas, texto, tempo y frases en un historial y SHALL permitir deshacer y rehacer al menos las últimas 100 operaciones.

#### Scenario: Deshacer un movimiento

- **WHEN** el usuario deshace tras mover una nota
- **THEN** la nota vuelve a su beat, duración y pitch anteriores

#### Scenario: Rehacer tras deshacer

- **WHEN** el usuario rehace inmediatamente después de deshacer
- **THEN** se reaplica el mismo cambio una sola vez

### Requirement: Guardado automático y cambios sin guardar

El sistema SHALL guardar automáticamente el estado editable y SHALL avisar antes de cerrar si quedan cambios pendientes.

#### Scenario: Autoguardado

- **WHEN** el usuario edita una nota y espera unos segundos
- **THEN** el cambio queda persistido en el proyecto sin acción adicional

#### Scenario: Cierre con cambios

- **WHEN** el usuario intenta cerrar la ventana con cambios sin guardar
- **THEN** el sistema ofrece guardar, descartar o cancelar el cierre

### Requirement: Exportación desde el editor

El sistema SHALL exportar la canción desde el editor y SHALL llevar al usuario a la nota concreta cuando una validación impida exportar.

#### Scenario: Error de validación

- **WHEN** una nota no tiene texto y la exportación falla
- **THEN** el editor selecciona esa nota y muestra el motivo

#### Scenario: Exportación correcta

- **WHEN** la validación pasa
- **THEN** el sistema escribe la carpeta de canción y muestra su ruta y el resultado

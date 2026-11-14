# Spec Delta

## Purpose

Prepara y mantiene los modelos locales de separación, transcripción y análisis, verificando su integridad y eligiendo el backend de ejecución adecuado.

## ADDED Requirements

### Requirement: Catálogo de modelos

El sistema SHALL mostrar los modelos necesarios con su finalidad, tamaño, versión, licencia, requisitos de hardware y estado local.

#### Scenario: Primera ejecución

- **WHEN** el usuario abre la app sin modelos descargados
- **THEN** ve el catálogo con el tamaño total y qué etapas quedarán bloqueadas hasta descargarlos

#### Scenario: Modelo ya presente

- **WHEN** un modelo está descargado y verificado
- **THEN** el catálogo lo marca como listo y muestra su ruta local

### Requirement: Consentimiento antes de descargar

El sistema SHALL pedir confirmación antes de iniciar cualquier descarga de modelo y SHALL permitir posponerla sin romper el proyecto.

#### Scenario: Descarga rechazada

- **WHEN** el usuario rechaza la descarga del modelo de separación
- **THEN** la etapa de separación queda bloqueada con un aviso y el resto de la interfaz sigue operativa

### Requirement: Descarga reanudable y verificación

El sistema SHALL descargar cada modelo con progreso, reanudación tras interrupción y verificación de integridad mediante hash antes de usarlo.

#### Scenario: Corte de red

- **WHEN** la descarga se interrumpe al 60 por ciento
- **THEN** el sistema conserva lo descargado y reanuda desde ese punto en el siguiente intento

#### Scenario: Hash incorrecto

- **WHEN** el archivo descargado no coincide con el hash esperado
- **THEN** el sistema lo marca como corrupto, no lo usa y ofrece volver a descargarlo

### Requirement: Almacenamiento configurable

El sistema SHALL guardar los modelos en un directorio de datos de la aplicación y SHALL permitir cambiarlo a otra ruta con espacio suficiente.

#### Scenario: Cambio de carpeta

- **WHEN** el usuario elige otra carpeta para modelos
- **THEN** el sistema valida permisos y espacio, mueve o vuelve a descargar los modelos y actualiza las rutas

#### Scenario: Espacio insuficiente

- **WHEN** la carpeta elegida no tiene espacio para el tamaño conocido del modelo
- **THEN** el sistema rechaza el cambio antes de descargar

### Requirement: Funcionamiento offline

El sistema SHALL operar sin red cuando todos los modelos requeridos por las etapas activas están presentes y verificados.

#### Scenario: Modo avión

- **WHEN** el equipo no tiene red y los modelos están completos
- **THEN** separación, transcripción y análisis funcionan con normalidad

#### Scenario: Modelo ausente sin red

- **WHEN** falta un modelo y no hay conexión
- **THEN** el sistema explica que no puede descargarlo y no deja la etapa en un bucle de reintentos

### Requirement: Selección y caída de backend

El sistema SHALL detectar el hardware disponible, SHALL elegir CPU o GPU según la configuración y SHALL degradar a CPU cuando el backend elegido falle por falta de memoria o compatibilidad.

#### Scenario: GPU disponible

- **WHEN** el equipo expone una GPU compatible y el usuario permite aceleración
- **THEN** el sistema la usa y lo indica en la interfaz

#### Scenario: Fallo de GPU

- **WHEN** el backend de GPU no puede inicializarse
- **THEN** el sistema continúa en CPU, informa del cambio y no bloquea el proyecto

### Requirement: Eliminación de modelos

El sistema SHALL permitir eliminar modelos locales y SHALL indicar qué etapas dejarán de funcionar hasta volver a descargarlos.

#### Scenario: Eliminar un modelo

- **WHEN** el usuario elimina el modelo de transcripción
- **THEN** el sistema borra solo los archivos de ese modelo y marca la transcripción como no disponible

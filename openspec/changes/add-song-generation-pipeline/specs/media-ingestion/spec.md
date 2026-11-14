# Spec Delta

## Purpose

Obtiene los medios de una canción desde una URL: metadatos, audio, carátula y vídeo opcional, dejándolos listos y normalizados para el análisis.

## ADDED Requirements

### Requirement: Comprobación de herramientas externas

El sistema SHALL comprobar antes de descargar que dispone de `yt-dlp`, `ffmpeg` y un runtime JavaScript compatible con YouTube, y SHALL indicar cómo resolver cualquier ausencia sin iniciar la descarga.

#### Scenario: Falta el runtime JavaScript

- **WHEN** `yt-dlp` está disponible pero no se detecta Deno ni Node
- **THEN** el sistema bloquea la descarga de YouTube y explica qué runtime instalar o configurar

#### Scenario: Herramientas disponibles

- **WHEN** las tres dependencias se detectan con versiones compatibles
- **THEN** el sistema habilita la descarga y muestra las versiones detectadas

### Requirement: Obtención de metadatos

El sistema SHALL obtener título, artista cuando exista, duración, miniatura y sitio de origen antes de descargar el audio, y SHALL permitir corregir título y artista.

#### Scenario: Metadatos completos

- **WHEN** el vídeo expone título y miniatura
- **THEN** el sistema rellena el proyecto con ambos y usa el título como nombre propuesto

#### Scenario: Artista ausente

- **WHEN** el vídeo no ofrece un artista identificable
- **THEN** el sistema deja el campo artista vacío y marca que debe completarse antes de exportar

### Requirement: Descarga de audio

El sistema SHALL descargar la mejor pista de audio disponible y SHALL conservar una copia de alta calidad para la exportación además de una copia WAV mono a 48 kHz para el análisis.

#### Scenario: Audio descargado

- **WHEN** la descarga termina correctamente
- **THEN** el proyecto contiene un audio reproducible y un WAV mono 48 kHz con la misma duración

#### Scenario: Fallo de red reanudable

- **WHEN** la conexión se interrumpe durante la descarga
- **THEN** el sistema reintenta de forma limitada y, si no lo consigue, conserva el estado para reanudar sin dejar archivos parciales utilizables

### Requirement: Descarga de carátula

El sistema SHALL obtener una carátula cuadrada a partir de la miniatura del vídeo y SHALL permitir sustituirla por un archivo local antes de exportar.

#### Scenario: Miniatura disponible

- **WHEN** el vídeo tiene miniatura
- **THEN** el sistema la recorta a formato cuadrado y la guarda junto al audio

#### Scenario: Sustitución manual

- **WHEN** el usuario elige una imagen local como carátula
- **THEN** el sistema valida que es una imagen legible y reemplaza la anterior

### Requirement: Vídeo opcional

El sistema SHALL permitir descargar el vídeo de origen como opción independiente y SHALL continuar el flujo si el vídeo no está disponible o el usuario lo desactiva.

#### Scenario: Vídeo desactivado

- **WHEN** el usuario desactiva la descarga de vídeo
- **THEN** el sistema no descarga vídeo y el resto del procesamiento continúa con audio y carátula

#### Scenario: Vídeo no disponible

- **WHEN** la fuente no ofrece un formato de vídeo descargable
- **THEN** el sistema registra un aviso, omite el vídeo y no falla el proyecto

### Requirement: Nombres de archivo seguros

El sistema SHALL generar nombres de archivo portables que eliminen caracteres reservados de Windows, macOS y Linux, y SHALL evitar colisiones dentro del proyecto.

#### Scenario: Título con caracteres reservados

- **WHEN** el título contiene `:`, `/`, `\`, `?`, `*` o comillas
- **THEN** el sistema los sustituye y conserva el título original solo en los metadatos

#### Scenario: Título repetido

- **WHEN** dos proyectos comparten el mismo título en el mismo destino
- **THEN** el sistema añade un sufijo estable para no sobrescribir archivos existentes

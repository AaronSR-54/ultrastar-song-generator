# Spec Delta

## Purpose

Publica los binarios de escritorio como GitHub Releases por plataforma y entrega al visitante el archivo correcto para su sistema, con URLs estables y verificación de integridad.

## ADDED Requirements

### Requirement: Publicación automática al etiquetar

El proyecto SHALL crear una GitHub Release al recibir un tag `v*`, con un asset por plataforma compatible y un archivo de sumas de verificación, solo cuando las tres compilaciones hayan terminado correctamente.

#### Scenario: Tag de versión

- **WHEN** se empuja un tag `v0.2.0`
- **THEN** existe una release `v0.2.0` con los tres ZIP, `SHA256SUMS.txt` y notas de la versión

#### Scenario: Compilación fallida

- **WHEN** una de las plataformas no compila o no supera el `--selftest`
- **THEN** no se publica la release y los assets no quedan accesibles como descarga pública

#### Scenario: Ejecución manual

- **WHEN** se lanza el workflow de release a mano sin un tag
- **THEN** se generan los artefactos de CI pero no se publica ninguna release

### Requirement: Assets por plataforma con nombres estables

Cada release SHALL incluir exactamente estos nombres de asset: `usdx-desktop-windows-x86_64.zip`, `usdx-desktop-macos-aarch64.zip`, `usdx-desktop-linux-x86_64.zip` y `SHA256SUMS.txt`. Los ZIP SHALL conservar el permiso de ejecución del binario en macOS y Linux.

#### Scenario: Descomprimir en Linux o macOS

- **WHEN** el visitante descomprime el ZIP de su plataforma
- **THEN** el binario ya es ejecutable y no requiere `chmod` manual

#### Scenario: Verificar la descarga

- **WHEN** el visitante ejecuta `sha256sum -c SHA256SUMS.txt` (o `shasum -a 256 -c` en macOS) junto a los archivos descargados
- **THEN** cada asset coincide con la suma publicada

### Requirement: URLs de descarga estables

Los assets SHALL ser accesibles mediante `https://github.com/AaronSR-54/ultrastar-song-generator/releases/latest/download/<asset>` y esas URLs SHALL seguir resolviendo a la última versión sin cambios en el sitio.

#### Scenario: Nueva versión publicada

- **WHEN** se publica una versión nueva con los mismos nombres de asset
- **THEN** las URLs de descarga del sitio sirven los archivos nuevos sin modificar ni redesplegar la landing

### Requirement: Detección de plataforma en el navegador

El sitio SHALL determinar plataforma y arquitectura con Client Hints cuando estén disponibles y con el user agent como alternativa, y SHALL mapear el resultado a un asset solo cuando exista una compilación compatible. La detección SHALL ejecutarse en el cliente, sin enviar los datos a ningún servidor propio ni a terceros.

#### Scenario: Client Hints disponibles

- **WHEN** el navegador ofrece `navigator.userAgentData` con plataforma y arquitectura
- **THEN** la recomendación usa esos valores antes que el user agent

#### Scenario: Navegador sin Client Hints

- **WHEN** el navegador solo expone `navigator.userAgent`
- **THEN** la recomendación se calcula con el user agent y, si es ambigua, se muestran todas las plataformas sin destacar un binario concreto

#### Scenario: iPadOS que se identifica como macOS

- **WHEN** el user agent parece macOS pero el dispositivo es táctil
- **THEN** no se recomienda el binario de macOS

#### Scenario: Arquitectura no soportada

- **WHEN** la plataforma detectada es Windows ARM64, Linux ARM o macOS Intel
- **THEN** el sitio no enlaza un binario incompatible y ofrece compilar desde el código fuente

#### Scenario: Sin conexión a la API de GitHub

- **WHEN** la detección se completa pero no se puede confirmar la existencia de la release
- **THEN** el sitio mantiene enlaces a la página de releases en lugar de presentar una descarga rota como válida

### Requirement: Integridad visible para el visitante

El sitio SHALL enlazar `SHA256SUMS.txt` y SHALL mostrar el comando de verificación correspondiente a cada plataforma junto a las descargas.

#### Scenario: Visitante quiere comprobar el archivo

- **WHEN** el visitante abre la sección de descargas
- **THEN** encuentra el archivo de sumas y un comando copiable para verificar su descarga

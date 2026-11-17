# Spec Delta

## Purpose

Define cómo se empaqueta y publica la aplicación de escritorio para que cada sistema operativo soportado tenga un instalador nativo, verificable y coherente con la versión del producto.

## ADDED Requirements

### Requirement: Instalador nativo por sistema operativo

El sistema SHALL producir un instalador nativo para cada plataforma soportada: un paquete MSI en Windows, una imagen DMG en macOS y un paquete Debian en Linux.

#### Scenario: Artefacto de Windows

- **WHEN** se empaqueta la versión de release para Windows x86_64
- **THEN** el resultado incluye un archivo `.msi` cuyo nombre contiene el nombre del producto y la versión

#### Scenario: Artefacto de macOS

- **WHEN** se empaqueta la versión de release para macOS arm64
- **THEN** el resultado incluye un archivo `.dmg` que contiene `UltraStar Song Generator.app`

#### Scenario: Artefacto de Linux

- **WHEN** se empaqueta la versión de release para Linux x86_64
- **THEN** el resultado incluye un archivo `.deb` cuya arquitectura declarada es `amd64`

### Requirement: Versión única del producto

El sistema SHALL obtener la versión de los instaladores de `workspace.package.version`, sin declararla de nuevo en los manifiestos de empaquetado.

#### Scenario: Cambio de versión del workspace

- **WHEN** la versión del workspace pasa a `0.2.0` y se empaqueta
- **THEN** el MSI, el DMG y el DEB declaran la versión `0.2.0` en sus metadatos

#### Scenario: Metadatos incoherentes

- **WHEN** un artefacto declara una versión distinta de la del workspace
- **THEN** la verificación de release falla y el artefacto no se publica

### Requirement: Instalación por usuario en Windows

El instalador de Windows SHALL instalar la aplicación en el perfil del usuario actual sin requerir privilegios de administrador ni mostrar UAC.

#### Scenario: Instalación sin elevación

- **WHEN** un usuario sin privilegios de administrador ejecuta el MSI
- **THEN** la aplicación queda instalada dentro de su perfil y el instalador termina con éxito

#### Scenario: Acceso desde el menú Inicio

- **WHEN** la instalación termina
- **THEN** aparece una entrada «UltraStar Song Generator» en el menú Inicio que abre la aplicación

#### Scenario: Entrada de desinstalación

- **WHEN** el usuario abre *Aplicaciones instaladas* en Windows
- **THEN** encuentra «UltraStar Song Generator» con su versión y puede desinstalarla desde ahí

#### Scenario: Ventana sin consola

- **WHEN** el usuario abre la aplicación instalada
- **THEN** solo aparece la ventana de la aplicación, sin ninguna ventana de consola

### Requirement: Imagen de disco en macOS

El instalador de macOS SHALL entregar la aplicación como un bundle `.app` con identificador, nombre visible, versión, icono y `Info.plist` válidos, acompañado de un enlace a *Applications*.

#### Scenario: Montaje del DMG

- **WHEN** el usuario abre el DMG
- **THEN** ve `UltraStar Song Generator.app` y un acceso directo a la carpeta *Applications*

#### Scenario: Ejecución desde Applications

- **WHEN** el usuario arrastra la app a *Applications* y la abre
- **THEN** la app arranca y muestra el nombre y el icono correctos en el Dock y en el menú de aplicaciones

### Requirement: Integración con el escritorio en Linux

El paquete Debian SHALL instalar el ejecutable, una entrada `.desktop` visible en el menú de aplicaciones y los iconos en el tema hicolor, bajo rutas del sistema y sin escribir en el directorio personal del usuario.

#### Scenario: Instalación con dpkg

- **WHEN** el usuario instala el `.deb` con `dpkg -i` o `apt install ./archivo.deb`
- **THEN** el comando `usdx-desktop` queda disponible en el `PATH` y la entrada de menú aparece sin reiniciar la sesión

#### Scenario: Lanzamiento desde el menú

- **WHEN** el usuario abre «UltraStar Song Generator» desde el menú de aplicaciones
- **THEN** se abre la ventana de la app y el lanzador usa el icono instalado

#### Scenario: Desinstalación

- **WHEN** el usuario desinstala el paquete
- **THEN** se eliminan el ejecutable, el lanzador, la entrada de menú y los iconos instalados

### Requirement: Datos del usuario intactos

Ningún instalador SHALL crear, modificar ni eliminar proyectos, ajustes o modelos de usuario, y la desinstalación SHALL conservarlos.

#### Scenario: Instalación sobre una app ya usada

- **WHEN** el usuario instala una versión nueva sobre una instalación existente que tiene proyectos y modelos
- **THEN** los proyectos, ajustes y modelos siguen disponibles al abrir la app

#### Scenario: Desinstalación con datos

- **WHEN** el usuario desinstala la app después de usarla
- **THEN** sus proyectos, ajustes y modelos permanecen en disco

### Requirement: Contenido del paquete

Los instaladores SHALL incluir solo la aplicación y sus recursos propios; no SHALL incluir `ffmpeg`, `yt-dlp`, intérpretes JavaScript ni pesos de modelos, y la app SHALL seguir avisando de las herramientas externas que falten.

#### Scenario: Inspección del paquete

- **WHEN** se inspecciona el contenido de cualquiera de los tres instaladores
- **THEN** no aparece ningún binario de `ffmpeg`, `yt-dlp`, Node/Deno ni ningún archivo de pesos de modelos

#### Scenario: Herramienta ausente tras instalar

- **WHEN** el usuario instala la app en un equipo sin `ffmpeg` y abre **Ajustes**
- **THEN** la app marca `ffmpeg` como ausente igual que en una ejecución desde el código fuente

### Requirement: Icono y nombre consistentes

Los tres instaladores SHALL usar el mismo nombre visible «UltraStar Song Generator» y un icono de aplicación presente en los tamaños que exige cada sistema operativo.

#### Scenario: Iconos de Windows

- **WHEN** se revisan el acceso directo del menú Inicio, la entrada de desinstalación y la barra de tareas
- **THEN** los tres muestran el icono de la aplicación, no el icono genérico del ejecutable

#### Scenario: Icono del bundle de macOS

- **WHEN** se abre el DMG
- **THEN** el `.app` muestra el icono de la aplicación y no el icono genérico de aplicación sin icono

#### Scenario: Iconos de Linux

- **WHEN** se inspecciona el paquete Debian
- **THEN** incluye iconos en los tamaños estándar de hicolor y la entrada `.desktop` apunta a uno de ellos

### Requirement: Verificación antes de publicar

El sistema SHALL comprobar cada instalador generado antes de publicarlo: que el binario empaquetado supera `--selftest` y que el artefacto se instala y desinstala sin error en el sistema de destino del job.

#### Scenario: Instalación real en CI

- **WHEN** termina el empaquetado de Linux en CI
- **THEN** el job instala el `.deb`, ejecuta `usdx-desktop --selftest`, comprueba `selftest ok` y desinstala el paquete

#### Scenario: Paquete roto

- **WHEN** el instalador no se instala o el binario empaquetado falla el selftest
- **THEN** el job falla y el artefacto no se adjunta al release

### Requirement: Publicación en el release

El sistema SHALL adjuntar los tres instaladores al GitHub Release del tag `v*` y SHALL publicarlos como artefactos descargables cuando el workflow se ejecute manualmente.

#### Scenario: Release por tag

- **WHEN** se empuja un tag que empieza por `v`
- **THEN** el release publicado contiene los tres instaladores con nombre, versión y arquitectura en el nombre del archivo

#### Scenario: Ejecución manual

- **WHEN** se lanza el workflow de release con `workflow_dispatch`
- **THEN** los tres instaladores quedan disponibles como artefactos de la ejecución aunque no se cree un release

### Requirement: Artefactos sin firmar documentados

Mientras los instaladores no estén firmados, el release SHALL incluir instrucciones de instalación que expliquen el aviso de SmartScreen en Windows y el bloqueo de Gatekeeper en macOS, y cómo continuar de forma consciente.

#### Scenario: Aviso en Windows

- **WHEN** el usuario descarga y ejecuta el MSI sin firma
- **THEN** dispone de instrucciones publicadas que explican el aviso de SmartScreen y cómo instalarlo igualmente

#### Scenario: Bloqueo en macOS

- **WHEN** el usuario abre el DMG sin firma en macOS
- **THEN** dispone de instrucciones publicadas que explican el bloqueo de Gatekeeper y la forma documentada de abrir la app

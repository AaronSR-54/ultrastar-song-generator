# Spec Delta

## Purpose

Página pública de producto, en español y desplegada en Vercel, que presenta la app con su misma identidad visual y permite descargar la versión adecuada para el sistema del visitante.

## ADDED Requirements

### Requirement: Landing pública en español

El sitio SHALL servir una única página en la raíz, en español, sin registro ni inicio de sesión, con la propuesta de valor, una llamada a la descarga y enlaces al repositorio y a la documentación.

#### Scenario: Visitante abre la raíz

- **WHEN** un visitante carga la URL raíz del sitio
- **THEN** ve el nombre del producto, una frase que resume «de enlace de YouTube a canción jugable», la descarga principal y enlaces a GitHub

#### Scenario: Navegación por secciones

- **WHEN** el visitante usa la navegación o los anclajes de la página
- **THEN** llega a las secciones de características, flujo, requisitos, descargas y preguntas frecuentes sin recargar

### Requirement: Identidad visual derivada de la app

El sitio SHALL usar la paleta y las reglas de la app de escritorio: fondo `#0A0A0C`, superficies `#15161B`/`#1B1C22`, borde `#2A2C36`, texto `#F4F4F7` y `#A0A2AD`, acento `#4F8DFF` con texto `#080D18`, esquinas rectas en superficies interactivas y retícula de puntos de fondo. El acento SHALL reservarse para lo accionable o lo que está en reproducción.

#### Scenario: Revisión de tokens

- **WHEN** se inspeccionan los estilos calculados de superficies, botones y texto
- **THEN** los colores y radios coinciden con los tokens de la app y no aparecen colores de acento adicionales

#### Scenario: Contraste legible

- **WHEN** se mide el contraste del texto sobre su superficie
- **THEN** el texto normal alcanza al menos 4.5:1 y el texto grande al menos 3:1

### Requirement: Descarga adaptada al sistema del visitante

El sitio SHALL destacar la descarga correspondiente al sistema operativo y la arquitectura detectados, SHALL ofrecer un selector manual con todas las plataformas compatibles y SHALL renderizar las tres descargas en el HTML inicial aunque JavaScript esté desactivado.

#### Scenario: Visitante en Windows x86_64

- **WHEN** la detección reconoce Windows de 64 bits
- **THEN** la llamada principal dice «Descargar para Windows» y enlaza al asset de Windows x86_64

#### Scenario: Visitante en macOS Apple Silicon

- **WHEN** la detección reconoce macOS con arquitectura ARM64
- **THEN** la llamada principal enlaza al asset de macOS aarch64

#### Scenario: Visitante en Linux x86_64

- **WHEN** la detección reconoce Linux de 64 bits
- **THEN** la llamada principal enlaza al asset de Linux x86_64

#### Scenario: Plataforma sin binario publicado

- **WHEN** el visitante usa iOS, Android, Windows ARM64, Linux ARM o macOS Intel
- **THEN** el sitio no ofrece un binario incorrecto: indica que no hay compilación para ese sistema y enlaza a las instrucciones de compilación y al resto de descargas

#### Scenario: Detección no bloqueante

- **WHEN** el visitante quiere otra plataforma distinta de la detectada
- **THEN** puede elegirla manualmente y descargarla sin cambiar de página

#### Scenario: Sin JavaScript

- **WHEN** el navegador no ejecuta JavaScript
- **THEN** las tres descargas siguen visibles y enlazadas con su plataforma etiquetada

### Requirement: Requisitos e instalación por plataforma

El sitio SHALL explicar que la app no incluye sus dependencias externas y SHALL listar `yt-dlp`, `ffmpeg`/`ffprobe`, un runtime de JavaScript y `espeak-ng` (opcional), junto con los pasos y avisos propios de Windows, macOS y Linux.

#### Scenario: Visitante revisa requisitos

- **WHEN** el visitante abre la sección de requisitos
- **THEN** ve las herramientas necesarias, que deben estar en el `PATH`, y la descarga de modelos en el primer uso (unos 620 MB, más Whisper si se transcribe)

#### Scenario: Avisos del sistema operativo

- **WHEN** el visitante consulta la instalación de su plataforma
- **THEN** Windows muestra el aviso de SmartScreen, macOS el de Gatekeeper y Linux la necesidad de dar permiso de ejecución y de bibliotecas de audio/ventana

### Requirement: Contenido fiel al producto

Todo texto del sitio SHALL describir funciones que existen en la app, sin prometer duetos, GPU, firma de código ni funciones futuras. Las afirmaciones SHALL ser trazables al `README.md` o a `docs/`.

#### Scenario: Revisión de afirmaciones

- **WHEN** se compara cada función anunciada con la documentación del repositorio
- **THEN** cada una está respaldada y no se anuncia nada marcado como fuera del alcance

#### Scenario: Privacidad del procesamiento

- **WHEN** el visitante lee la sección de privacidad
- **THEN** entiende que el audio se procesa en local y que solo el título, el artista y la duración salen a la red para buscar la letra

### Requirement: Responsive y accesible

El sitio SHALL funcionar desde 320 px de ancho sin desplazamiento horizontal, SHALL ser navegable con teclado con foco visible, SHALL usar HTML semántico con un solo `h1`, y SHALL respetar `prefers-reduced-motion`.

#### Scenario: Móvil estrecho

- **WHEN** se carga el sitio en una ventana de 320 px
- **THEN** el contenido se reordena en una columna, la descarga principal queda alcanzable y no aparece scroll horizontal

#### Scenario: Navegación solo con teclado

- **WHEN** el visitante recorre la página con `Tab` y activa con `Enter`
- **THEN** puede usar la descarga, el selector de plataforma y las preguntas frecuentes viendo siempre el anillo de foco

#### Scenario: Movimiento reducido

- **WHEN** el sistema declara `prefers-reduced-motion: reduce`
- **THEN** no hay animaciones de entrada, desplazamientos automáticos ni transiciones largas

### Requirement: Rendimiento y metadatos sociales

El sitio SHALL prerenderizar el contenido principal, SHALL evitar scripts de terceros que bloqueen el renderizado y SHALL publicar `lang="es"`, título, descripción, canónica e imagen Open Graph.

#### Scenario: Auditoría móvil

- **WHEN** se ejecuta Lighthouse móvil sobre el despliegue de Vercel
- **THEN** Rendimiento ≥ 95 y Accesibilidad, Buenas prácticas y SEO = 100, con LCP < 2.5 s y CLS < 0.1

#### Scenario: Enlace compartido

- **WHEN** se pega la URL en GitHub, X o WhatsApp
- **THEN** la vista previa muestra el título, la descripción y la imagen Open Graph del producto

### Requirement: Verificación del sitio en CI

Los cambios que toquen `site/` SHALL pasar instalación reproducible, lint, comprobación de tipos y build de producción antes de integrarse.

#### Scenario: Pull request con error de tipos

- **WHEN** un pull request modifica `site/` e introduce un error de TypeScript
- **THEN** el workflow del sitio falla y bloquea la integración

#### Scenario: Cambio ajeno al sitio

- **WHEN** un pull request solo modifica crates Rust
- **THEN** el workflow del sitio no se ejecuta

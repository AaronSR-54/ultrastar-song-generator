# Proposal

## Why

El proyecto solo se puede obtener compilando con Rust y el README está escrito para desarrolladores: no hay una vía pública de descarga ni una página que explique qué hace la app y qué necesita el equipo. Una landing en Vercel, con la misma identidad visual que la app, convierte el repositorio en un producto descargable y elimina la fricción de elegir el binario correcto.

## What Changes

- Nueva aplicación **Next.js 16 (App Router, TypeScript, Tailwind CSS 4)** en `site/`, desplegada en Vercel con `site/` como *root directory*, en español y sin backend propio.
- Landing de una página que reutiliza el lenguaje visual de la app (fondo casi negro, superficies elevadas, acento azul único, esquinas rectas, retícula de puntos, micro-etiquetas mono, bordes de 1 px) con una calidad visual de portfolio: composición propia, demo del flujo dibujada con HTML/SVG y movimiento sobrio.
- Detección en cliente del sistema operativo y la arquitectura para destacar la descarga correcta (Windows x86_64, macOS aarch64, Linux x86_64), con selector manual para el resto de casos y sin bloquear plataformas.
- `release.yml` publica una **GitHub Release** en cada tag `v*` con archivos ZIP por plataforma y un `SHA256SUMS.txt`, usando nombres de asset estables bajo `releases/latest/download/<asset>`.
- Nuevo workflow de CI para el sitio (`install`, `lint`, `typecheck`, `build`) que solo se dispara cuando cambia `site/`.
- Documentación de despliegue y de los nombres de asset en `site/README.md`; `.gitignore` cubre los artefactos de Node/Next.

## Capabilities

### New Capabilities

- `landing-page`: página pública de producto en Vercel que explica la app, respeta su identidad visual, es responsive y accesible, resume requisitos y muestra el estado de las descargas.
- `app-distribution`: publicación automatizada de binarios por plataforma como GitHub Release con URLs estables y resolución de la descarga según el sistema del visitante.

### Modified Capabilities

<!-- Ninguna: no hay specs existentes y no cambia el comportamiento de la app de escritorio. -->

## Impact

- **Nuevo:** `site/` (app Next.js, componentes, estilos y tests ligeros), `site/README.md`.
- **Modificado:** `.github/workflows/release.yml` (job de publicación), `.gitignore`.
- **Nuevo:** `.github/workflows/site.yml`.
- **Sin cambios:** crates Rust, formato `.txt`, pipeline y binario de escritorio.
- **Dependencias:** solo dentro de `site/` (Next 16, React 19, Tailwind CSS 4, TypeScript). El workspace Cargo no cambia.
- **Infraestructura:** proyecto de Vercel apuntando al repositorio con `site/` como raíz; `GITHUB_TOKEN` con permiso `contents: write` para crear la release.
- **Riesgo:** los binarios no están firmados ni notarizados; la landing debe explicar los avisos de SmartScreen/Gatekeeper y los requisitos externos (`yt-dlp`, `ffmpeg`, Node, `espeak-ng`).

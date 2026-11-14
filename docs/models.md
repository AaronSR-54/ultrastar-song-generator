# Modelos locales

La app no incluye los modelos en el repositorio. Los descarga bajo demanda, con consentimiento, y los verifica antes de usarlos.

## Catálogo

| Id | Uso | Tamaño | Licencia |
|---|---|---|---|
| `htdemucs` | Separación de voz e instrumental | 316 446 953 B (~302 MB) | Pesos Demucs para uso científico; export ONNX MIT |
| `whisper-large-v3-turbo-q5` | Transcripción | 574 041 195 B (~547 MB) | MIT |

Total aproximado: **849 MB**.

Cada entrada del catálogo (`crates/application/assets/models.toml`) incluye id, nombre, finalidad, archivo, URL HTTPS, tamaño, SHA-256, licencia y requisitos. `ModelCatalog::validate` rechaza URLs no HTTPS, hashes que no sean 64 caracteres hexadecimales, nombres de archivo con rutas, licencias vacías y duplicados.

Para inspeccionar el catálogo y el estado local:

```bash
cargo run -p usdx-application --example models
```

## Consentimiento

Antes de descargar, la app muestra:

- Qué modelo se necesita y para qué etapa.
- Tamaño de la descarga y espacio libre.
- Licencia y aviso de uso.
- Acciones **Descargar** y **Ahora no**.

Rechazar la descarga bloquea la etapa correspondiente, pero no rompe el proyecto ni el resto de la interfaz. No se inicia ninguna descarga automática.

## Almacenamiento

Ruta por defecto, vía `directories::ProjectDirs`:

| Sistema | Ruta |
|---|---|
| Windows | `%APPDATA%\ultrastar-song-generator\models` |
| macOS | `~/Library/Application Support/ultrastar-song-generator/models` |
| Linux | `~/.local/share/ultrastar-song-generator/models` |

La ruta se puede cambiar desde la configuración. `ModelStore::check_space` comprueba el espacio disponible con `fs4` antes de descargar y rechaza la operación si no cabe. `ModelStore::delete` borra el modelo y su `.part`; la app indica qué etapas dejan de estar disponibles.

## Descarga, reanudación e integridad

- Se escribe en `<modelo>.part` y se renombra al finalizar.
- Si existe un `.part`, se reanuda con la cabecera `Range: bytes=N-`.
- Si el servidor ignora el rango y devuelve `200`, se reinicia el archivo.
- Al terminar se calcula SHA-256; si no coincide, se borra el `.part` y se devuelve `HashMismatch`.
- La cancelación conserva el `.part` para reanudar más tarde.
- Un `416 Range Not Satisfiable` reinicia la descarga desde cero.

## Modo offline

Si todos los modelos requeridos por las etapas activas están presentes y verificados, separación y transcripción funcionan sin red. Si falta un modelo y no hay conexión, la app lo explica y no entra en un bucle de reintentos.

## Hardware y backends

`Hardware::detect` comprueba:

- **CUDA** si `nvidia-smi` responde con una GPU.
- **CoreML** en macOS.

`choose_backend` acepta `Auto`, `Cpu` o `Gpu`:

1. Prueba el backend preferido (CUDA o CoreML).
2. Si falla por memoria o incompatibilidad, cae a CPU y muestra el motivo.
3. Si no hay GPU, usa CPU directamente.

El MVP distribuye la build CPU; las builds con GPU se documentan aparte. La selección es inyectable (`try_init`) para poder probar el fallback sin hardware real.

## Licencias

- **Whisper**: pesos MIT, descargados del repositorio oficial de whisper.cpp.
- **HTDemucs**: el código es MIT, pero los pesos no están cubiertos por esa licencia y se ofrecen para uso científico. No se redistribuyen. La app muestra el aviso y el usuario decide. Para un producto comercial hay que sustituirlos por pesos con licencia compatible.
- Consulta [licenses.md](licenses.md) para la matriz completa.

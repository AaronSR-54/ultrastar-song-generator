# Separación de voz

Aísla la voz cantada del instrumental con HTDemucs sobre ONNX Runtime, en local y sin Python.

## Motor

- Implementación: `charon-audio` 0.1.2 (MIT/Apache-2.0), que reproduce el pipeline de Demucs 4.1: STFT/iSTFT, ventanas, overlap-add y normalización.
- Modelo: `htdemucs.onnx`, 316 446 953 bytes, SHA-256 `68d0bf16428ef66e692cdff8a9ccf28f1ef3f69440d57e58605a4cc55fcc5e74`.
- Backend: ONNX Runtime (CPU por defecto; CUDA/CoreML cuando la build lo habilita).
- Memoria: el modelo ocupa ~302 MB en disco y el proceso necesita del orden de 1–2 GB de RAM según la duración.
- Licencia: el código de Demucs es MIT, pero los pesos no; se ofrecen para uso científico y no se redistribuyen. Ver [licenses.md](licenses.md).

## Flujo

1. Leer el WAV de análisis (`work/audio-48k-mono.wav`) con `AudioFile::read`.
2. Ejecutar `Separator::separate_with` con un `Control` que reporta progreso por ventanas y comprueba cancelación.
3. Obtener los stems `drums`, `bass`, `other` y `vocals`.
4. Calcular el instrumental como la suma de `drums + bass + other`. Si el modelo no devuelve esos stems, se usa `mezcla - vocals`.
5. Re-muestrear cada stem a la frecuencia del audio original si hizo falta y ajustar la longitud exacta (recorte o relleno con ceros).
6. Escribir `work/stems/vocals.wav` e `work/stems/instrumental.wav` como WAV float32.

Los stems conservan canales, frecuencia y número de muestras del audio de entrada. La suma de ambos se aproxima a la mezcla; el test real exige un error relativo menor que 0.25.

## Progreso y cancelación

`SeparationControl` comparte un `CancelFlag` con el resto de la app y un callback de progreso. Como `charon-audio` usa su propio `CancelToken`, el adaptador lanza un hilo puente que lo activa cuando el flag de la app se cancela. El progreso se convierte de ventanas a fracción 0..1.

Cancelar devuelve `SeparationError::Cancelled`, elimina los stems parciales de esa ejecución y deja la etapa pendiente. La UI no debe marcar la etapa como fallida.

## Fallback de memoria

`SeparationService` recibe un motor primario y uno de respaldo. Si el primario falla con un mensaje de memoria (`out of memory`, `OOM`, `memory allocation`), reintenta en el respaldo (CPU) y devuelve el motivo para mostrarlo. Otros errores no se reintentan.

El test simula un backend que agota memoria y comprueba que el fallback se usa; también cubre la cancelación cooperativa.

## Energía vocal

`analyze_vocal_energy` mide:

- RMS global.
- Pico.
- Fracción de tramas de 50 ms por encima de 0.005.

`VocalEnergy::is_voice_missing` marca voz insuficiente si el RMS es menor que 0.003 o la fracción sonora es menor que 0.02. La UI muestra un aviso y sugiere revisar la fuente o continuar con transcripción manual.

## Verificación real

El test `crates/ml/tests/separation.rs` se ejecuta con:

```bash
export USDX_HTDEMUCS_MODEL=/ruta/a/htdemucs.onnx
cargo test -p usdx-ml --test separation -- --nocapture
```

Mezcla sintética de 6 s, 44,1 kHz, estéreo (voz de 440 Hz + tonos de 220/330 Hz):

- Dos ejecuciones completas en ~20 s (≈10 s por ejecución).
- `vocals.wav` e `instrumental.wav` con 6 s, 44,1 kHz, 2 canales.
- Error relativo de reconstrucción < 0.25.
- Los archivos de las dos ejecuciones son **idénticos byte a byte** (determinismo).

## Windows y ONNX Runtime

`ort-sys` con `copy-dylibs` tiene un bug en Windows: intenta copiar las DLL a `target/<perfil>/build/examples` y `target/<perfil>/build/deps`, que no existen, y el build falla con `path not found`.

Antes de compilar, crea esas carpetas:

```powershell
New-Item -ItemType Directory -Force target\debug\build\examples,target\debug\build\deps
```

Para ejecutar tests o la app, añade `target\<perfil>\build` al `PATH` para que Windows encuentre `DirectML.dll`:

```powershell
$env:Path = "$PWD\target\debug\build;$env:Path"
```

CI hace ambas cosas antes de compilar. En el empaquetado final hay que copiar esas DLL junto al ejecutable o cargar ONNX Runtime de forma dinámica.

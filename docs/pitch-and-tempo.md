# Pitch, tempo y cuantización

Convierte la pista vocal en notas con altura y duración, estima el tempo y las ajusta a la rejilla de beats de UltraStar.

## Seguimiento de F0

- Motor: `pitch-core` con pYIN (probabilistic YIN).
- Entrada: voz mono; el tracker remuestrea a 48 kHz.
- Ventana: 2048 muestras (~43 ms), hop de 960 muestras (20 ms).
- Rango: 65–1100 Hz (C2–C6).
- Salida por trama: tiempo, Hz, confianza de sonoridad y flag `voiced`.
- Una trama es válida si `hz > 0` y `confidence >= 0.25`.

pYIN procesa lotes de 16 000 muestras; el análisis rellena el final con ceros hasta completar el lote para no perder los últimos 333 ms.

## Segmentación en notas

1. Convertir Hz a MIDI fraccional.
2. Filtro de mediana de 5 tramas para eliminar saltos aislados.
3. Agrupar tramas sonoras; un silencio de más de 50 ms cierra el grupo.
4. Dentro de un grupo, dividir cuando la altura se aparta al menos **1 semitono** de la referencia de la nota durante **3 tramas** (60 ms).
5. Descartar notas de menos de **80 ms**.
6. La altura final es la mediana de la nota, redondeada a semitono y limitada a `-60..=67`.

El vibrato de ±0.3 semitonos no se fragmenta; un cambio sostenido de semitono sí.

## Mapeo a UltraStar

- `pitch = round(midi) - 60`; por tanto `0` es C4 y `9` es A4 (440 Hz).
- Las notas fuera de rango se recortan a `PITCH_MIN`/`PITCH_MAX` y se registra el ajuste.
- La frecuencia media original se conserva en `PitchNote::freq_hz`.

## Tempo y GAP

1. Remuestrear a 22,05 kHz.
2. Calcular una envolvente de onsets por flujo espectral (FFT de 1024, hop 512, ventana Hann).
3. Autocorrelación de la envolvente para lags entre 60 y 180 BPM.
4. Seleccionar máximos locales y generar candidatos, incluyendo mitad y doble.
5. Aplicar un prior suave centrado en 100 BPM para evitar errores de octava.
6. Para cada candidato, buscar la fase que maximiza la energía de la rejilla; esa fase es el `#GAP` en ms.
7. `#BPM` exportado = `bpm_musical * 4`.

El resultado (`TempoEstimate`) incluye el mejor candidato, hasta cuatro alternativas y una confianza. La UI debe mostrar los candidatos para que el usuario confirme mitad/doble.

## Cuantización

`quantize` convierte cada nota a beats enteros con la fórmula del formato:

```
beat = round((t_ms - GAP) * BPM * 4 / 60000)
```

Reglas:

- `start_beat >= 0`.
- `length_beats >= 1`.
- Orden cronológico.
- Sin solapes:
  - Si dos notas empiezan en el mismo beat, se conserva la de mayor confianza.
  - Si se solapan, la de menor confianza se desplaza al final de la anterior.
  - Si la nueva tiene más confianza, se recorta la anterior.
- Cada ajuste queda en `adjustments` con el índice y el motivo.

## Reproducibilidad

El test `analysis_is_deterministic_json` ejecuta dos veces el análisis y compara el JSON byte a byte. El test de tempo usa clics sintéticos a 80, 100 y 120 BPM y exige que el mejor candidato esté a menos de 2 BPM.

## Verificación

```bash
cargo test -p usdx-ml --lib pitch_analysis -- --nocapture
cargo test -p usdx-ml --lib tempo -- --nocapture
cargo test -p usdx-ml --lib quantize -- --nocapture
```

Los tests generan tonos y clics en WAV temporal; no requieren red ni modelos.

## Ajuste manual

Desde el editor se puede cambiar `#BPM` y `#GAP`; `quantize` se vuelve a ejecutar sobre las notas en segundos sin repetir pYIN. Un BPM mayor da más resolución (beats más cortos) pero también más sensibilidad a la deriva de tempo. El formato no admite mapa de tempo.

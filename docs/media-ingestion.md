# Ingesta de medios

Convierte una URL en los medios locales de un proyecto: metadatos, audio, carátula y vídeo.

## Herramientas

La app detecta y valida al arrancar:

| Herramienta | Versión mínima | Uso |
|---|---|---|
| `yt-dlp` | 2024.01.01 | Descarga y metadatos |
| `ffmpeg` | 4.0 | Conversión de audio y vídeo |
| `ffprobe` | 4.0 | Comprobación de duración, canales y frecuencia |
| Deno | 2.0.0 | Runtime JavaScript recomendado para YouTube |
| Node | 18.0.0 | Alternativa a Deno |

`ffprobe` suele estar junto a `ffmpeg`; si no está en `PATH`, la app lo busca en la misma carpeta.

`yt-dlp` necesita un runtime JavaScript para resolver los desafíos de YouTube. La app añade `--js-runtimes deno` o `--js-runtimes node` **solo si la versión de yt-dlp soporta esa opción**. Las versiones antiguas no la conocen y fallan con `no such option: --js-runtimes`; para YouTube conviene mantener yt-dlp actualizado. La versión validada es **2026.10.21** con Node 22.

Puedes comprobar el estado con:

```bash
cargo run -p usdx-media --example toolchain
```

## Flujo

1. **Metadatos**: `yt-dlp --dump-single-json --skip-download --no-playlist URL`.
   - Título obligatorio; artista se toma de `artist`, `creator`, `uploader` o `channel`.
   - Duración en `duration` (segundos) o `duration_ms`.
   - Miniatura en `thumbnail` o en la última entrada de `thumbnails`.
   - Las playlists se rechazan.
2. **Audio**: `yt-dlp -f bestaudio/best -x --audio-format mp3 --audio-quality 0 -o source/audio-source.%(ext)s URL`.
   - Se obtiene `source/audio-source.mp3` a alta calidad.
3. **WAV de análisis**: `ffmpeg -i audio-source.mp3 -vn -ac 1 -ar 48000 -c:a pcm_s16le work/audio-48k-mono.wav`.
   - Se verifica que sea mono, 48 kHz y que la duración no difiera más de 250 ms del original.
4. **Carátula**: se descarga la miniatura, se recorta al centro en cuadrado y se guarda como `source/cover.jpg` a 800×800. Se puede sustituir por una imagen local; se valida y se vuelve a recortar.
5. **Vídeo**: `yt-dlp -f "bv*[height<=1080][vcodec^=avc1]/bv*[height<=1080]/..." --merge-output-format mp4
   --remux-video mp4 -o source/video.%(ext)s URL`.
   - La app lo descarga para cada canción nueva, porque UltraStar lo muestra durante el juego.
   - Se pide **solo vídeo, sin pista de audio**: la partida usa `audio.mp3`, y así no se descargan
     megabytes que nadie va a oír.
   - Se prefiere **H.264 hasta 1080p**, el códec que mejor digieren los reproductores de UltraStar;
     si el sitio no lo ofrece, se acepta otra variante y se remuxe a `.mp4`.
   - El resultado queda como `source/video.mp4` en el proyecto.
   - Si falla o no hay formato de vídeo, se registra un aviso y el proyecto continúa sin vídeo. En
     **Canción lista** aparece **Añadir vídeo**, que lo descarga después y vuelve a exportar.
6. **Nombres**: los títulos se sanean para Windows/macOS/Linux (caracteres reservados, nombres `CON`, `NUL`, etc.) y se evitan colisiones con sufijos ` (2)`, ` (3)`.

## Timeouts, cancelación y limpieza

- Metadatos: 120 s.
- Descarga de audio: 30 min.
- Conversión: 10 min.
- La cancelación es cooperativa y mata el proceso hijo (`kill_on_drop`).
- Al fallar una descarga se eliminan `*.part`, `*.ytdl` y `*.temp` del prefijo correspondiente. Nunca se deja un archivo parcial como si fuera definitivo.

## Prueba real registrada

Con `https://www.youtube.com/watch?v=jNQXAC9IVRw` y `USDX_VIDEO=1`:

```bash
cargo run -p usdx-media --example ingest_smoke -- https://www.youtube.com/watch?v=jNQXAC9IVRw /tmp/ingest-smoke
```

Resultado:

- Título: `Me at the zoo`, artista `jawed`, duración 19 000 ms.
- `audio-source.mp3`: 331 052 bytes.
- `cover.jpg`: 116 901 bytes, 800×800.
- `video.mp4`: 433 081 bytes, solo vídeo H.264 320×240, 18,9 s (sin pista de audio).
- Sonda del audio original: 19 006 ms, 48 kHz, 2 canales.
- Sonda del WAV de análisis: 19 006 ms, 48 kHz, 1 canal.

## Errores frecuentes

| Mensaje | Causa | Solución |
|---|---|---|
| `no such option: --js-runtimes` | yt-dlp antiguo | Actualiza yt-dlp; la app omite la opción en versiones viejas |
| `No supported JavaScript runtime could be found` | Falta Deno/Node | Instala Deno 2+ o Node 18+ |
| `The page needs to be reloaded` | Desafío de YouTube no resuelto | Actualiza yt-dlp, comprueba el runtime JS y, si hace falta, usa cookies |
| `This video is unavailable` | Vídeo privado, eliminado o restringido | Prueba otra URL |
| `ffmpeg was not found in PATH` | ffmpeg no instalado | Instala ffmpeg y reinicia la terminal |
| `playlists are not supported` | URL de playlist | Usa la URL de un vídeo concreto |

## Privacidad y uso

La descarga y el análisis son locales. Solo se descargan binarios/modelos y el contenido solicitado. Respeta los términos de YouTube y usa la herramienta para contenido propio o con permiso.

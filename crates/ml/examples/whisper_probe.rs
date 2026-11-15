//! Diagnostic runner for the local Whisper engine.
//!
//! ```text
//! USDX_WHISPER_MODEL=model.bin cargo run -p usdx-ml --example whisper_probe -- vocals.wav [seconds] [threads]
//! ```

use std::sync::Arc;
use std::time::Instant;

use usdx_ml::{load_vocals_16k, TranscriptionOptions, TranscriptionService, WhisperEngine};
use usdx_ports::{CancelFlag, TranscriptionControl};

fn main() {
    let mut args = std::env::args().skip(1);
    let audio = args
        .next()
        .expect("usage: whisper_probe <wav> [seconds] [threads]");
    let seconds: Option<f64> = args.next().and_then(|value| value.parse().ok());
    let threads: Option<usize> = args.next().and_then(|value| value.parse().ok());
    let model = std::env::var("USDX_WHISPER_MODEL").expect("USDX_WHISPER_MODEL");

    let mut samples = load_vocals_16k(&audio).expect("load audio");
    if let Some(seconds) = seconds {
        let limit = (seconds * 16_000.0) as usize;
        samples.truncate(limit.min(samples.len()));
    }
    eprintln!(
        "audio: {} samples ({:.1}s) threads={:?}",
        samples.len(),
        samples.len() as f64 / 16_000.0,
        threads
    );

    let mut options = TranscriptionOptions::default();
    if let Some(threads) = threads {
        options.threads = threads;
    }

    let started = Instant::now();
    let engine = WhisperEngine::new(&model).expect("load model");
    eprintln!("model loaded in {:.1}s", started.elapsed().as_secs_f64());

    let service = TranscriptionService::new(engine, options);
    let control = TranscriptionControl::new(CancelFlag::default(), Arc::new(|_| {}));
    let started = Instant::now();
    match service.transcribe_samples(&samples, 16_000, Some("es"), control) {
        Ok(transcript) => {
            eprintln!(
                "ok in {:.1}s: {} lines, {} words",
                started.elapsed().as_secs_f64(),
                transcript.lines.len(),
                transcript.word_count()
            );
            for line in transcript.lines.iter().take(5) {
                let words: Vec<String> = line
                    .words
                    .iter()
                    .map(|word| format!("{}[{:.2}-{:.2}]", word.text, word.start_s, word.end_s))
                    .collect();
                println!("{:.2}-{:.2}: {}", line.start_s, line.end_s, words.join(" "));
            }
        }
        Err(error) => {
            eprintln!("failed in {:.1}s: {error}", started.elapsed().as_secs_f64());
            std::process::exit(1);
        }
    }
}

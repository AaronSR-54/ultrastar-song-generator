use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use usdx_domain::Transcript;
use usdx_ml::{
    load_vocals_16k, transcription_capabilities, AsrEngine, TranscriptionOptions,
    TranscriptionService, WhisperEngine, WHISPER_SAMPLE_RATE,
};
use usdx_ports::{CancelFlag, Transcriber, TranscriptionControl};

fn control() -> TranscriptionControl {
    TranscriptionControl::new(CancelFlag::default(), Arc::new(|_| {}))
}

fn model_path() -> Option<PathBuf> {
    std::env::var_os("USDX_WHISPER_MODEL").map(PathBuf::from)
}

fn fixture(name: &str) -> Option<PathBuf> {
    std::env::var_os(name).map(PathBuf::from)
}

fn assert_monotonic(transcript: &Transcript) {
    let mut previous_end = f64::NEG_INFINITY;
    for line in &transcript.lines {
        assert!(
            line.start_s <= line.end_s,
            "line bounds are inverted: {line:?}"
        );
        for word in &line.words {
            assert!(word.start_s >= previous_end, "word times are not monotonic");
            assert!(
                word.end_s > word.start_s,
                "word has non-positive duration: {word:?}"
            );
            previous_end = word.end_s;
        }
    }
}

/// Single entry point for the optional real-model test.
///
/// It is ignored by default because it needs a 547 MiB model and two voice
/// fixtures. Run it explicitly with `--ignored --nocapture`. It is one test
/// function on purpose: whisper.cpp is heavy and several inference tests in
/// the same process fight over the machine.
#[test]
#[ignore = "requires USDX_WHISPER_MODEL and USDX_WHISPER_AUDIO_ES/EN"]
fn real_model_end_to_end() {
    let Some(model) = model_path() else {
        eprintln!("USDX_WHISPER_MODEL is not set; skipping real transcription test");
        return;
    };
    let (Some(spanish), Some(english)) = (
        fixture("USDX_WHISPER_AUDIO_ES"),
        fixture("USDX_WHISPER_AUDIO_EN"),
    ) else {
        eprintln!("USDX_WHISPER_AUDIO_ES/EN are not set; skipping real transcription test");
        return;
    };

    let capabilities = transcription_capabilities();
    assert!(capabilities.runs_locally);
    assert!(capabilities
        .languages
        .iter()
        .any(|language| language.code == "es"));
    assert!(capabilities
        .languages
        .iter()
        .any(|language| language.code == "en"));

    let samples = load_vocals_16k(&spanish).expect("load vocals");
    assert!(!samples.is_empty());
    assert!(samples.len() > WHISPER_SAMPLE_RATE as usize);
    assert!(samples.iter().all(|sample| sample.is_finite()));

    let started = Instant::now();
    let es = {
        let engine = WhisperEngine::new(&model).expect("load whisper model");
        let error = engine
            .transcribe(
                &[0.0; 1000],
                48_000,
                Some("es"),
                &TranscriptionOptions::default(),
                &control(),
            )
            .unwrap_err();
        assert!(
            error.to_string().contains("16000"),
            "wrong sample rate must be rejected: {error}"
        );
        let transcript = TranscriptionService::new(&engine, TranscriptionOptions::default())
            .transcribe(&spanish, Some("es"), control())
            .expect("transcribe Spanish");
        drop(engine);
        transcript
    };
    let es_elapsed = started.elapsed();

    let started = Instant::now();
    let en = {
        let engine = WhisperEngine::new(&model).expect("load whisper model");
        let transcript = TranscriptionService::new(&engine, TranscriptionOptions::default())
            .transcribe(&english, Some("en"), control())
            .expect("transcribe English");
        drop(engine);
        transcript
    };
    let en_elapsed = started.elapsed();

    let started = Instant::now();
    let detected = {
        let engine = WhisperEngine::new(&model).expect("load whisper model");
        let transcript = TranscriptionService::new(&engine, TranscriptionOptions::default())
            .transcribe(&spanish, None, control())
            .expect("detect language without forcing it");
        drop(engine);
        transcript
    };
    let detect_elapsed = started.elapsed();

    eprintln!(
        "es: {:?} ({:.1}s) | en: {:?} ({:.1}s) | detected {} ({:.3}) ({:.1}s)",
        es.text(),
        es_elapsed.as_secs_f64(),
        en.text(),
        en_elapsed.as_secs_f64(),
        detected.language,
        detected.language_confidence,
        detect_elapsed.as_secs_f64()
    );

    assert_eq!(es.language, "es");
    assert_eq!(en.language, "en");
    assert_eq!(detected.language, "es");
    assert!(detected.language_confidence > 0.3);

    assert!(
        es.word_count() >= 5,
        "Spanish transcript is too short: {:?}",
        es.text()
    );
    assert!(
        en.word_count() >= 5,
        "English transcript is too short: {:?}",
        en.text()
    );
    assert!(
        es.text().to_lowercase().contains("hola"),
        "unexpected Spanish text: {:?}",
        es.text()
    );
    assert!(
        en.text().to_lowercase().contains("hello"),
        "unexpected English text: {:?}",
        en.text()
    );
    assert!(es.discarded_segments == 0, "{:?}", es);
    assert!(en.discarded_segments == 0, "{:?}", en);
    assert_monotonic(&es);
    assert_monotonic(&en);
    assert_monotonic(&detected);
    assert!(es.words().any(|word| word.confidence > 0.5));
    assert!(en.words().any(|word| word.confidence > 0.5));
}

//! Local transcription with Whisper (whisper.cpp through `whisper-rs`).

use std::path::{Path, PathBuf};
use std::sync::Arc;

use charon_audio::{AudioBuffer, AudioFile};
use usdx_domain::{Transcript, TranscriptLine, TranscriptWord};
use usdx_ports::{Transcriber, TranscriptionControl, TranscriptionError};
use whisper_rs::{
    FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters, WhisperState,
};

use crate::hallucination::{
    filter_segments, EnergyTrack, HallucinationOptions, RawSegment, RawWord,
};

/// Whisper always works at 16 kHz mono, as required by the model.
pub const WHISPER_SAMPLE_RATE: u32 = 16_000;
/// Minimum word duration enforced when timestamps collide.
pub const MIN_WORD_SECONDS: f64 = 0.02;

#[derive(Debug, Clone, PartialEq)]
pub struct TranscriptionOptions {
    pub threads: usize,
    pub translate: bool,
    /// Words below this probability are flagged for review.
    pub low_confidence_threshold: f32,
    pub hallucination: HallucinationOptions,
}

impl Default for TranscriptionOptions {
    fn default() -> Self {
        let threads = std::thread::available_parallelism()
            .map(|value| value.get().min(8))
            .unwrap_or(4);
        Self {
            threads,
            translate: false,
            low_confidence_threshold: 0.55,
            hallucination: HallucinationOptions::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AsrOutput {
    /// ISO-639-1 code, e.g. `es`.
    pub language: String,
    pub language_confidence: f32,
    pub segments: Vec<RawSegment>,
}

/// Speech recognition backend. Implemented by Whisper and by test doubles.
pub trait AsrEngine: Send + Sync {
    fn transcribe(
        &self,
        samples: &[f32],
        sample_rate: u32,
        language: Option<&str>,
        options: &TranscriptionOptions,
        control: &TranscriptionControl,
    ) -> Result<AsrOutput, TranscriptionError>;
}

/// Allows sharing one loaded engine behind a reference.
impl<T: AsrEngine + ?Sized> AsrEngine for &T {
    fn transcribe(
        &self,
        samples: &[f32],
        sample_rate: u32,
        language: Option<&str>,
        options: &TranscriptionOptions,
        control: &TranscriptionControl,
    ) -> Result<AsrOutput, TranscriptionError> {
        (**self).transcribe(samples, sample_rate, language, options, control)
    }
}

/// In-process Whisper engine.
pub struct WhisperEngine {
    context: WhisperContext,
    model_path: PathBuf,
}

impl WhisperEngine {
    pub fn new(model_path: impl Into<PathBuf>) -> Result<Self, TranscriptionError> {
        Self::build(model_path, false)
    }

    pub fn new_with_gpu(model_path: impl Into<PathBuf>) -> Result<Self, TranscriptionError> {
        Self::build(model_path, true)
    }

    fn build(model_path: impl Into<PathBuf>, use_gpu: bool) -> Result<Self, TranscriptionError> {
        let model_path = model_path.into();
        if !model_path.exists() {
            return Err(TranscriptionError::Failed(format!(
                "whisper model not found: {}",
                model_path.display()
            )));
        }
        let mut parameters = WhisperContextParameters::default();
        parameters.use_gpu(use_gpu);
        let context = WhisperContext::new_with_params(&model_path, parameters)
            .map_err(|error| TranscriptionError::Failed(error.to_string()))?;
        Ok(Self {
            context,
            model_path,
        })
    }

    pub fn model_path(&self) -> &Path {
        &self.model_path
    }
}

/// whisper.cpp occasionally reports `-6` (failed to encode) on a healthy
/// machine and succeeds when the same audio is encoded again with a fresh
/// state. Treating it as fatal would fail whole songs at random, so the engine
/// retries it once.
pub const MAX_ENCODE_ATTEMPTS: usize = 5;

/// True when a whisper error is the transient encoder failure.
pub fn is_transient_whisper_failure(error: &TranscriptionError) -> bool {
    match error {
        TranscriptionError::Failed(message) => {
            message.contains("Error code: -6") || message.contains("failed to encode")
        }
        _ => false,
    }
}

impl AsrEngine for WhisperEngine {
    fn transcribe(
        &self,
        samples: &[f32],
        sample_rate: u32,
        language: Option<&str>,
        options: &TranscriptionOptions,
        control: &TranscriptionControl,
    ) -> Result<AsrOutput, TranscriptionError> {
        if samples.is_empty() {
            return Err(TranscriptionError::EmptyAudio);
        }
        if sample_rate != WHISPER_SAMPLE_RATE {
            return Err(TranscriptionError::Failed(format!(
                "whisper expects {WHISPER_SAMPLE_RATE} Hz audio, got {sample_rate} Hz"
            )));
        }

        let mut attempt = 1;
        loop {
            match self.transcribe_once(samples, language, options, control) {
                Ok(output) => return Ok(output),
                Err(error)
                    if attempt < MAX_ENCODE_ATTEMPTS
                        && !control.is_cancelled()
                        && is_transient_whisper_failure(&error) =>
                {
                    tracing::warn!(
                        %error,
                        attempt,
                        "whisper encoder failed; retrying with a fresh state"
                    );
                    attempt += 1;
                }
                Err(error) => return Err(error),
            }
        }
    }
}

impl WhisperEngine {
    fn transcribe_once(
        &self,
        samples: &[f32],
        language: Option<&str>,
        options: &TranscriptionOptions,
        control: &TranscriptionControl,
    ) -> Result<AsrOutput, TranscriptionError> {
        if control.is_cancelled() {
            return Err(TranscriptionError::Cancelled);
        }

        let mut state = self
            .context
            .create_state()
            .map_err(|error| TranscriptionError::Failed(error.to_string()))?;
        state
            .pcm_to_mel(samples, options.threads)
            .map_err(|error| TranscriptionError::Failed(error.to_string()))?;

        let (language, language_confidence) = resolve_language(&state, language, options.threads)?;
        if control.is_cancelled() {
            return Err(TranscriptionError::Cancelled);
        }

        let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
        params.set_n_threads(options.threads as i32);
        params.set_translate(options.translate);
        params.set_no_context(true);
        params.set_token_timestamps(true);
        params.set_no_speech_thold(options.hallucination.max_no_speech_probability);
        if language == "und" {
            params.set_detect_language(true);
        } else {
            params.set_language(Some(&language));
        }

        let progress = Arc::clone(&control.progress);
        params.set_progress_callback_safe(move |value| {
            progress((value as f32 / 100.0) * 0.85);
        });
        let cancel = control.cancel.clone();
        params.set_abort_callback_safe(move || cancel.is_cancelled());

        let result = state.full(params, samples);
        if control.is_cancelled() {
            return Err(TranscriptionError::Cancelled);
        }
        result.map_err(|error| TranscriptionError::Failed(error.to_string()))?;

        let segments = raw_segments(&state);
        Ok(AsrOutput {
            language,
            language_confidence,
            segments,
        })
    }
}

fn resolve_language(
    state: &WhisperState,
    forced: Option<&str>,
    threads: usize,
) -> Result<(String, f32), TranscriptionError> {
    if let Some(language) = forced {
        let normalized = language.trim().to_lowercase();
        if whisper_rs::get_lang_id(&normalized).is_none() {
            return Err(TranscriptionError::UnsupportedLanguage(language.into()));
        }
        return Ok((normalized, 1.0));
    }

    match state.lang_detect(0, threads) {
        Ok((id, probabilities)) => {
            let language = whisper_rs::get_lang_str(id).unwrap_or("und").to_string();
            let confidence = probabilities
                .get(id.max(0) as usize)
                .copied()
                .unwrap_or(0.0);
            Ok((language, confidence.clamp(0.0, 1.0)))
        }
        Err(error) => {
            tracing::warn!(%error, "whisper language detection failed; continuing without it");
            Ok(("und".to_string(), 0.0))
        }
    }
}

struct WordBuilder {
    text: String,
    start_s: f64,
    end_s: f64,
    probabilities: Vec<f32>,
}

impl WordBuilder {
    fn new(start_s: f64, end_s: f64) -> Self {
        Self {
            text: String::new(),
            start_s,
            end_s,
            probabilities: Vec::new(),
        }
    }

    fn push(&mut self, text: &str, start_s: f64, end_s: f64, probability: f32) {
        self.text.push_str(text);
        self.start_s = self.start_s.min(start_s);
        self.end_s = self.end_s.max(end_s);
        self.probabilities.push(probability);
    }

    fn finish(self) -> Option<RawWord> {
        let text = self.text.split_whitespace().collect::<Vec<_>>().join(" ");
        if text.is_empty() {
            return None;
        }
        let confidence = if self.probabilities.is_empty() {
            0.0
        } else {
            self.probabilities.iter().sum::<f32>() / self.probabilities.len() as f32
        };
        Some(RawWord {
            text,
            start_s: self.start_s,
            end_s: self.end_s.max(self.start_s + MIN_WORD_SECONDS),
            confidence,
        })
    }
}

fn raw_segments(state: &WhisperState) -> Vec<RawSegment> {
    let mut segments = Vec::new();
    for segment in state.as_iter() {
        let segment_start = segment.start_timestamp() as f64 / 100.0;
        let segment_end = segment.end_timestamp() as f64 / 100.0;
        let mut words: Vec<RawWord> = Vec::new();
        let mut builder: Option<WordBuilder> = None;

        for token_index in 0..segment.n_tokens() {
            let Some(token) = segment.get_token(token_index) else {
                continue;
            };
            let text = token
                .to_str_lossy()
                .map(|text| text.into_owned())
                .unwrap_or_default();
            if is_special_token(&text) {
                continue;
            }
            let data = token.token_data();
            let start_s = if data.t0 >= 0 {
                data.t0 as f64 / 100.0
            } else {
                segment_start
            };
            let end_s = if data.t1 >= 0 {
                data.t1 as f64 / 100.0
            } else {
                start_s + MIN_WORD_SECONDS
            }
            .max(start_s + MIN_WORD_SECONDS);

            let starts_word = text.starts_with(char::is_whitespace);
            let trimmed = text.trim();
            if trimmed.is_empty() {
                continue;
            }

            if starts_word {
                if let Some(word) = builder.take().and_then(WordBuilder::finish) {
                    words.push(word);
                }
                builder = Some(WordBuilder::new(start_s, end_s));
                if let Some(builder) = builder.as_mut() {
                    builder.push(trimmed, start_s, end_s, data.p);
                }
            } else if let Some(builder) = builder.as_mut() {
                builder.push(trimmed, start_s, end_s, data.p);
            } else {
                let mut new_builder = WordBuilder::new(start_s, end_s);
                new_builder.push(trimmed, start_s, end_s, data.p);
                builder = Some(new_builder);
            }
        }
        if let Some(word) = builder.take().and_then(WordBuilder::finish) {
            words.push(word);
        }

        if words.is_empty() {
            continue;
        }
        make_monotonic(&mut words);
        let start_s = segment_start.min(words[0].start_s);
        let end_s = segment_end.max(words[words.len() - 1].end_s);
        segments.push(RawSegment {
            start_s,
            end_s,
            no_speech_probability: segment.no_speech_probability(),
            words,
        });
    }
    segments
}

fn is_special_token(text: &str) -> bool {
    let trimmed = text.trim();
    (trimmed.starts_with("<|") && trimmed.ends_with("|>"))
        || (trimmed.starts_with('[') && trimmed.ends_with(']'))
        || trimmed.chars().all(|character| character == '♪')
}

/// Forces non-decreasing word times without changing text or confidence.
pub fn make_monotonic(words: &mut [RawWord]) {
    let mut previous_end = f64::NEG_INFINITY;
    for word in words.iter_mut() {
        word.start_s = word.start_s.max(previous_end).max(0.0);
        word.end_s = word.end_s.max(word.start_s + MIN_WORD_SECONDS);
        previous_end = word.end_s;
    }
}

/// Builds the persisted transcript from filtered segments.
pub fn build_transcript(
    output: AsrOutput,
    filtered: Vec<RawSegment>,
    discarded_segments: usize,
    low_confidence_threshold: f32,
) -> Transcript {
    let lines = filtered
        .into_iter()
        .filter_map(|segment| {
            let words: Vec<TranscriptWord> = segment
                .words
                .into_iter()
                .filter(|word| !word.text.trim().is_empty())
                .map(|word| TranscriptWord {
                    text: word.text,
                    start_s: word.start_s,
                    end_s: word.end_s,
                    confidence: word.confidence,
                    low_confidence: word.confidence < low_confidence_threshold,
                    syllables: Vec::new(),
                })
                .collect();
            (!words.is_empty()).then(|| TranscriptLine::new(words))
        })
        .collect();

    Transcript {
        language: output.language,
        language_confidence: output.language_confidence,
        lines,
        discarded_segments,
        word_timed: true,
    }
}

/// Loads the vocal stem as 16 kHz mono samples for Whisper.
pub fn load_vocals_16k(path: impl AsRef<Path>) -> Result<Vec<f32>, TranscriptionError> {
    let audio = AudioFile::read(path.as_ref())
        .map_err(|error| TranscriptionError::Failed(error.to_string()))?;
    let mono = AudioBuffer::new(audio.to_mono(), audio.sample_rate);
    let resampled = mono
        .resample(WHISPER_SAMPLE_RATE)
        .map_err(|error| TranscriptionError::Failed(error.to_string()))?;
    if resampled.samples() == 0 {
        return Err(TranscriptionError::EmptyAudio);
    }
    Ok(resampled.data.row(0).to_vec())
}

/// Model shipped in the catalog for transcription.
pub const WHISPER_MODEL_ID: &str = "whisper-large-v3-turbo-q5";
/// Size of `ggml-large-v3-turbo-q5_0.bin` in bytes.
pub const WHISPER_MODEL_SIZE_BYTES: u64 = 574_041_195;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LanguageInfo {
    pub code: &'static str,
    pub name: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TranscriptionCapabilities {
    pub model_id: &'static str,
    pub model_size_bytes: u64,
    pub languages: Vec<LanguageInfo>,
    pub runs_locally: bool,
}

/// Languages exposed by the linked whisper.cpp build. The GUI renders this
/// list, so the documentation cannot drift from the binary.
pub fn whisper_languages() -> Vec<LanguageInfo> {
    let max_id = whisper_rs::get_lang_max_id();
    (0..=max_id)
        .filter_map(|id| {
            Some(LanguageInfo {
                code: whisper_rs::get_lang_str(id)?,
                name: whisper_rs::get_lang_str_full(id)?,
            })
        })
        .collect()
}

pub fn transcription_capabilities() -> TranscriptionCapabilities {
    TranscriptionCapabilities {
        model_id: WHISPER_MODEL_ID,
        model_size_bytes: WHISPER_MODEL_SIZE_BYTES,
        languages: whisper_languages(),
        runs_locally: true,
    }
}

/// Wires an ASR engine to audio loading, hallucination filtering and the
/// persisted transcript model.
pub struct TranscriptionService<E> {
    engine: E,
    options: TranscriptionOptions,
}

impl<E: AsrEngine> TranscriptionService<E> {
    pub fn new(engine: E, options: TranscriptionOptions) -> Self {
        Self { engine, options }
    }

    pub fn engine(&self) -> &E {
        &self.engine
    }

    pub fn options(&self) -> &TranscriptionOptions {
        &self.options
    }

    pub fn transcribe_file(
        &self,
        vocals: impl AsRef<Path>,
        language: Option<&str>,
        control: TranscriptionControl,
    ) -> Result<Transcript, TranscriptionError> {
        if control.is_cancelled() {
            return Err(TranscriptionError::Cancelled);
        }
        let samples = load_vocals_16k(vocals)?;
        self.transcribe_samples(&samples, WHISPER_SAMPLE_RATE, language, control)
    }

    pub fn transcribe_samples(
        &self,
        samples: &[f32],
        sample_rate: u32,
        language: Option<&str>,
        control: TranscriptionControl,
    ) -> Result<Transcript, TranscriptionError> {
        if control.is_cancelled() {
            return Err(TranscriptionError::Cancelled);
        }
        let output =
            self.engine
                .transcribe(samples, sample_rate, language, &self.options, &control)?;
        if control.is_cancelled() {
            return Err(TranscriptionError::Cancelled);
        }

        control.report(0.9);
        let track = EnergyTrack::analyze(samples, sample_rate);
        let filtered = filter_segments(&output.segments, &track, self.options.hallucination);
        let discarded = filtered.report.total();
        if discarded > 0 {
            tracing::info!(
                discarded,
                silent = filtered.report.silent,
                no_speech = filtered.report.no_speech,
                repeated = filtered.report.repeated,
                known_phrase = filtered.report.known_phrase,
                "discarded transcription segments"
            );
        }
        control.report(1.0);
        Ok(build_transcript(
            output,
            filtered.segments,
            discarded,
            self.options.low_confidence_threshold,
        ))
    }
}

impl<E: AsrEngine> Transcriber for TranscriptionService<E> {
    fn transcribe(
        &self,
        vocals: &Path,
        language: Option<&str>,
        control: TranscriptionControl,
    ) -> Result<Transcript, TranscriptionError> {
        self.transcribe_file(vocals, language, control)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hallucination::EnergyTrack;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Mutex;
    use usdx_ports::CancelFlag;

    struct FakeEngine {
        output: AsrOutput,
        calls: AtomicUsize,
        seen_language: Mutex<Option<String>>,
    }

    impl FakeEngine {
        fn new(output: AsrOutput) -> Self {
            Self {
                output,
                calls: AtomicUsize::new(0),
                seen_language: Mutex::new(None),
            }
        }
    }

    impl AsrEngine for FakeEngine {
        fn transcribe(
            &self,
            _samples: &[f32],
            _sample_rate: u32,
            language: Option<&str>,
            _options: &TranscriptionOptions,
            _control: &TranscriptionControl,
        ) -> Result<AsrOutput, TranscriptionError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            *self.seen_language.lock().unwrap() = language.map(str::to_string);
            Ok(self.output.clone())
        }
    }

    fn segment(start_s: f64, end_s: f64, words: &[(&str, f64, f64, f32)]) -> RawSegment {
        RawSegment {
            start_s,
            end_s,
            no_speech_probability: 0.05,
            words: words
                .iter()
                .map(|(text, word_start, word_end, confidence)| RawWord {
                    text: text.to_string(),
                    start_s: *word_start,
                    end_s: *word_end,
                    confidence: *confidence,
                })
                .collect(),
        }
    }

    fn output() -> AsrOutput {
        AsrOutput {
            language: "es".into(),
            language_confidence: 0.93,
            segments: vec![segment(
                0.2,
                1.2,
                &[("Hola", 0.2, 0.6, 0.9), ("mundo", 0.7, 1.1, 0.4)],
            )],
        }
    }

    fn tone(seconds: f64) -> Vec<f32> {
        let samples = (16_000.0 * seconds) as usize;
        (0..samples)
            .map(|index| {
                let time = index as f32 / 16_000.0;
                (2.0 * std::f32::consts::PI * 220.0 * time).sin() * 0.2
            })
            .collect()
    }

    fn control() -> TranscriptionControl {
        TranscriptionControl::new(CancelFlag::default(), Arc::new(|_| {}))
    }

    #[test]
    fn builds_transcript_with_confidence_and_low_confidence_flags() {
        let transcript = build_transcript(output(), output().segments, 0, 0.55);
        assert_eq!(transcript.language, "es");
        assert_eq!(transcript.word_count(), 2);
        assert_eq!(transcript.lines[0].words[0].text, "Hola");
        assert!(!transcript.lines[0].words[0].low_confidence);
        assert!(transcript.lines[0].words[1].low_confidence);
        assert_eq!(transcript.lines[0].start_s, 0.2);
        assert_eq!(transcript.lines[0].end_s, 1.1);
        assert_eq!(transcript.discarded_segments, 0);
    }

    #[test]
    fn words_are_monotonic() {
        let mut words = vec![
            RawWord {
                text: "a".into(),
                start_s: 0.5,
                end_s: 0.9,
                confidence: 0.9,
            },
            RawWord {
                text: "b".into(),
                start_s: 0.6,
                end_s: 0.7,
                confidence: 0.9,
            },
            RawWord {
                text: "c".into(),
                start_s: 0.8,
                end_s: 0.8,
                confidence: 0.9,
            },
        ];
        make_monotonic(&mut words);
        assert!(words[1].start_s >= words[0].end_s);
        assert!(words[2].start_s >= words[1].end_s);
        assert!(words[2].end_s > words[2].start_s);
    }

    #[test]
    fn service_filters_silence_and_keeps_voiced_segments() {
        let engine = FakeEngine::new(AsrOutput {
            language: "en".into(),
            language_confidence: 0.8,
            segments: vec![
                segment(0.1, 0.6, &[("invented", 0.1, 0.4, 0.9)]),
                segment(2.7, 3.1, &[("real", 2.7, 3.0, 0.9)]),
            ],
        });
        let service = TranscriptionService::new(engine, TranscriptionOptions::default());

        let mut samples = vec![0.0_f32; 16_000];
        samples.extend(tone(0.6));
        samples.extend(vec![0.0_f32; 16_000]);
        samples.extend(tone(0.6));

        let transcript = service
            .transcribe_samples(&samples, 16_000, None, control())
            .unwrap();
        assert_eq!(transcript.word_count(), 1);
        assert_eq!(transcript.text(), "real");
        assert_eq!(transcript.discarded_segments, 1);
        assert_eq!(service.engine().calls.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn language_override_reaches_the_engine() {
        let engine = FakeEngine::new(AsrOutput {
            language: "en".into(),
            language_confidence: 1.0,
            segments: Vec::new(),
        });
        let service = TranscriptionService::new(engine, TranscriptionOptions::default());
        service
            .transcribe_samples(&tone(0.5), 16_000, Some("en"), control())
            .unwrap();
        assert_eq!(
            service.engine().seen_language.lock().unwrap().as_deref(),
            Some("en")
        );
    }

    #[test]
    fn pre_cancelled_service_never_calls_the_engine() {
        let engine = FakeEngine::new(output());
        let service = TranscriptionService::new(engine, TranscriptionOptions::default());
        let cancel = CancelFlag::default();
        cancel.cancel();
        let control = TranscriptionControl::new(cancel, Arc::new(|_| {}));
        let error = service
            .transcribe_samples(&tone(0.5), 16_000, None, control)
            .unwrap_err();
        assert!(matches!(error, TranscriptionError::Cancelled));
        assert_eq!(service.engine().calls.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn unsupported_language_is_rejected_before_inference() {
        let value = whisper_rs::get_lang_id("definitely-not-a-language");
        assert!(value.is_none());
    }

    #[test]
    fn energy_track_sees_the_synthetic_tone() {
        let samples = tone(1.0);
        let track = EnergyTrack::analyze(&samples, 16_000);
        let stats = track.stats(0.0, 1.0);
        assert!(stats.rms > 0.1);
        assert!(stats.voiced_fraction > 0.8);
    }

    #[test]
    fn only_encoder_failures_are_retried() {
        assert!(is_transient_whisper_failure(&TranscriptionError::Failed(
            "Generic whisper error. Varies depending on the function. Error code: -6".into()
        )));
        assert!(is_transient_whisper_failure(&TranscriptionError::Failed(
            "whisper_full_with_state: failed to encode".into()
        )));
        assert!(!is_transient_whisper_failure(&TranscriptionError::Failed(
            "Generic whisper error. Varies depending on the function. Error code: -7".into()
        )));
        assert!(!is_transient_whisper_failure(
            &TranscriptionError::Cancelled
        ));
        assert!(!is_transient_whisper_failure(
            &TranscriptionError::UnsupportedLanguage("xx".into())
        ));
        assert_eq!(MAX_ENCODE_ATTEMPTS, 5);
    }
}

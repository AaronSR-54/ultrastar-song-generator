//! Local inference adapters: source separation, transcription and pitch tracking.

pub mod alignment;
pub mod hallucination;
pub mod pitch_analysis;
pub mod quantize;
pub mod separation;
pub mod separation_service;
pub mod syllabify;
pub mod tempo;
pub mod transcript_edit;
pub mod transcription;
pub mod vocal_energy;

pub use alignment::{
    align, AlignedNote, AlignedPhrase, Alignment, AlignmentOptions, AlignmentStats,
    CONTINUATION_TEXT,
};
pub use hallucination::{
    filter_segments, normalize_text, DiscardReport, EnergyTrack, FilterOutcome,
    HallucinationOptions, RawSegment, RawWord, SegmentStats,
};
pub use pitch_analysis::{
    analyze_file, analyze_file_with, segment_notes, PitchAnalysis, PitchFrame, PitchNote,
    PitchOptions,
};
pub use quantize::{quantize, QuantizationAdjustment, QuantizedNote, QuantizedSong};
pub use separation::CharonSeparator;
pub use separation_service::SeparationService;
pub use syllabify::{
    apply_spanish_rules, dictionary_syllabifier, fallback_split, DictLanguage,
    DictionarySyllabifier,
};
pub use tempo::{estimate_tempo, TempoCandidate, TempoEstimate};
pub use transcript_edit::{
    correct_word, delete_line, delete_word, merge_line_with_next, reapply_syllables, split_line,
    Syllabifier, SyllableSplit, TranscriptEditError,
};
pub use transcription::{
    build_transcript, is_transient_whisper_failure, load_vocals_16k, make_monotonic,
    transcription_capabilities, whisper_languages, AsrEngine, AsrOutput, LanguageInfo,
    TranscriptionCapabilities, TranscriptionOptions, TranscriptionService, WhisperEngine,
    MAX_ENCODE_ATTEMPTS, WHISPER_MODEL_ID, WHISPER_MODEL_SIZE_BYTES, WHISPER_SAMPLE_RATE,
};
pub use vocal_energy::{analyze_vocal_energy, VocalEnergy};

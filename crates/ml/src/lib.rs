//! Local inference adapters: source separation, transcription and pitch tracking.

pub mod pitch_analysis;
pub mod quantize;
pub mod separation;
pub mod separation_service;
pub mod tempo;
pub mod vocal_energy;

pub use pitch_analysis::{
    analyze_file, analyze_file_with, segment_notes, PitchAnalysis, PitchFrame, PitchNote,
    PitchOptions,
};
pub use quantize::{quantize, QuantizationAdjustment, QuantizedNote, QuantizedSong};
pub use separation::CharonSeparator;
pub use separation_service::SeparationService;
pub use tempo::{estimate_tempo, TempoCandidate, TempoEstimate};
pub use vocal_energy::{analyze_vocal_energy, VocalEnergy};

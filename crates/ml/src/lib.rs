//! Local inference adapters: source separation, transcription and pitch tracking.

pub mod separation;
pub mod separation_service;
pub mod vocal_energy;

pub use separation::CharonSeparator;
pub use separation_service::SeparationService;
pub use vocal_energy::{analyze_vocal_energy, VocalEnergy};

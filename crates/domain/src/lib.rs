//! Core entities and rules for UltraStar song generation.
//!
//! This crate must stay independent from the GUI, media tooling and ML runtimes.

mod lyrics;
mod pitch;
mod song;

pub use lyrics::Syllable;
pub use pitch::{
    clamp_pitch, hz_to_midi, hz_to_pitch, midi_to_hz, pitch_to_midi, PITCH_MAX, PITCH_MIN,
};
pub use song::{Beat, Note, NoteKind, Phrase, PhraseBreak, Pitch, SongDoc, SongHeader};

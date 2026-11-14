//! Core entities and rules for UltraStar song generation.
//!
//! This crate must stay independent from the GUI, media tooling and ML runtimes.

mod lyrics;
mod song;

pub use lyrics::Syllable;
pub use song::{Beat, Note, NoteKind, Phrase, PhraseBreak, Pitch, SongDoc, SongHeader};

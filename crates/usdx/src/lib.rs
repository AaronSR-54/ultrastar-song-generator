//! Reader, writer and validator for the UltraStar `.txt` song format.

mod parser;
mod timing;
mod validation;
mod writer;

pub use parser::{parse, parse_bytes, parse_file, ParseResult, ParseWarning};
pub use timing::{
    beat_to_ms, clamp_pitch, hz_to_midi, hz_to_pitch, midi_to_hz, ms_to_beat, pitch_to_midi,
    PITCH_MAX, PITCH_MIN,
};
pub use validation::{validate, Severity, ValidationIssue, ValidationReport};
pub use writer::{format_decimal, write, write_file, WriteError};

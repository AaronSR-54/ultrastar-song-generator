use serde::{Deserialize, Serialize};
use usdx_domain::SongHeader;
use usdx_format::ms_to_beat;

use crate::pitch_analysis::PitchNote;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QuantizedNote {
    pub start_beat: u32,
    pub length_beats: u32,
    pub pitch: i16,
    pub confidence: f32,
    pub adjusted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QuantizationAdjustment {
    pub note_index: usize,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QuantizedSong {
    pub notes: Vec<QuantizedNote>,
    pub adjustments: Vec<QuantizationAdjustment>,
}

/// Converts seconds-based notes to integer UltraStar beats, enforcing a minimum
/// length of one beat and removing overlaps.
pub fn quantize(notes: &[PitchNote], header: &SongHeader) -> QuantizedSong {
    let mut quantized: Vec<QuantizedNote> = notes
        .iter()
        .map(|note| {
            let start = beat_for(header, note.start_s).max(0.0);
            let end = beat_for(header, note.end_s).max(start);
            let start_beat = start.round() as u32;
            let mut length_beats = (end.round() as i64 - start_beat as i64).max(0) as u32;
            if length_beats == 0 {
                length_beats = 1;
            }
            QuantizedNote {
                start_beat,
                length_beats,
                pitch: note.pitch,
                confidence: note.confidence,
                adjusted: false,
            }
        })
        .collect();

    quantized.sort_by_key(|note| note.start_beat);
    let mut adjustments = Vec::new();
    let mut resolved: Vec<QuantizedNote> = Vec::with_capacity(quantized.len());

    for (index, mut note) in quantized.into_iter().enumerate() {
        let Some(previous) = resolved.last_mut() else {
            resolved.push(note);
            continue;
        };

        let previous_end = previous.start_beat.saturating_add(previous.length_beats);
        if note.start_beat >= previous_end {
            resolved.push(note);
            continue;
        }

        if note.start_beat == previous.start_beat {
            if note.confidence > previous.confidence {
                *previous = note;
                adjustments.push(QuantizationAdjustment {
                    note_index: index,
                    reason: "replaced a same-beat note with higher confidence".into(),
                });
            } else {
                adjustments.push(QuantizationAdjustment {
                    note_index: index,
                    reason: "merged into a same-beat note with higher confidence".into(),
                });
            }
            continue;
        }

        if note.confidence < previous.confidence {
            let current_end = note.start_beat + note.length_beats;
            note.start_beat = previous_end;
            note.length_beats = current_end.saturating_sub(previous_end).max(1);
            note.adjusted = true;
            adjustments.push(QuantizationAdjustment {
                note_index: index,
                reason: "shifted to avoid overlapping a higher-confidence note".into(),
            });
            resolved.push(note);
        } else {
            previous.length_beats = note.start_beat.saturating_sub(previous.start_beat).max(1);
            previous.adjusted = true;
            adjustments.push(QuantizationAdjustment {
                note_index: index.saturating_sub(1),
                reason: "length trimmed by a higher-confidence note".into(),
            });
            resolved.push(note);
        }
    }

    QuantizedSong {
        notes: resolved,
        adjustments,
    }
}

fn beat_for(header: &SongHeader, seconds: f64) -> f64 {
    ms_to_beat(header, seconds * 1000.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn header() -> SongHeader {
        SongHeader {
            bpm: 120.0,
            gap_ms: 0,
            ..SongHeader::default()
        }
    }

    fn note(start_s: f64, end_s: f64, pitch: i16, confidence: f32) -> PitchNote {
        PitchNote {
            start_s,
            end_s,
            pitch,
            freq_hz: 440.0,
            confidence,
        }
    }

    #[test]
    fn converts_seconds_to_beats() {
        let song = quantize(&[note(0.0, 0.5, 9, 0.9)], &header());
        assert_eq!(song.notes[0].start_beat, 0);
        assert_eq!(song.notes[0].length_beats, 4);
        assert!(!song.notes[0].adjusted);
    }

    #[test]
    fn enforces_minimum_length() {
        let song = quantize(&[note(0.0, 0.01, 9, 0.9)], &header());
        assert_eq!(song.notes[0].length_beats, 1);
    }

    #[test]
    fn overlapping_notes_are_resolved() {
        let song = quantize(&[note(0.0, 0.5, 9, 0.9), note(0.4, 0.9, 9, 0.4)], &header());
        assert_eq!(song.notes.len(), 2);
        let first_end = song.notes[0].start_beat + song.notes[0].length_beats;
        assert!(song.notes[1].start_beat >= first_end);
        assert!(!song.adjustments.is_empty());
    }

    #[test]
    fn higher_confidence_note_wins_the_overlap() {
        let song = quantize(&[note(0.0, 0.5, 9, 0.2), note(0.4, 0.9, 9, 0.9)], &header());
        let first_end = song.notes[0].start_beat + song.notes[0].length_beats;
        assert!(song.notes[1].start_beat >= first_end);
        assert!(song.notes[0].adjusted);
    }

    #[test]
    fn no_overlaps_after_quantization() {
        let notes: Vec<_> = (0..20)
            .map(|index| {
                let start = index as f64 * 0.12;
                note(start, start + 0.4, index as i16 % 20 - 10, 0.5)
            })
            .collect();
        let song = quantize(&notes, &header());
        for pair in song.notes.windows(2) {
            let end = pair[0].start_beat + pair[0].length_beats;
            assert!(pair[1].start_beat >= end);
        }
    }
}

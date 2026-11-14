use std::fmt;

use usdx_domain::{SongDoc, SongHeader};

use crate::timing::{PITCH_MAX, PITCH_MIN};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationIssue {
    pub severity: Severity,
    pub phrase: Option<usize>,
    pub note: Option<usize>,
    pub message: String,
}

impl ValidationIssue {
    fn error(message: impl Into<String>) -> Self {
        Self {
            severity: Severity::Error,
            phrase: None,
            note: None,
            message: message.into(),
        }
    }

    fn note_error(phrase: usize, note: usize, message: impl Into<String>) -> Self {
        Self {
            severity: Severity::Error,
            phrase: Some(phrase),
            note: Some(note),
            message: message.into(),
        }
    }
}

impl fmt::Display for ValidationIssue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match (self.phrase, self.note) {
            (Some(phrase), Some(note)) => {
                write!(f, "phrase {phrase}, note {note}: {}", self.message)
            }
            (Some(phrase), None) => write!(f, "phrase {phrase}: {}", self.message),
            _ => write!(f, "{}", self.message),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ValidationReport {
    pub issues: Vec<ValidationIssue>,
}

impl ValidationReport {
    pub fn is_valid(&self) -> bool {
        !self
            .issues
            .iter()
            .any(|issue| issue.severity == Severity::Error)
    }

    pub fn errors(&self) -> impl Iterator<Item = &ValidationIssue> {
        self.issues
            .iter()
            .filter(|issue| issue.severity == Severity::Error)
    }

    pub fn first_error(&self) -> Option<&ValidationIssue> {
        self.errors().next()
    }
}

/// Validates a document before export.
///
/// Errors block export; warnings are informative. Issues carry phrase and note
/// indices whenever they point at a specific note.
pub fn validate(doc: &SongDoc) -> ValidationReport {
    let mut report = ValidationReport::default();

    if doc.is_duet {
        report
            .issues
            .push(ValidationIssue::error("duet songs are not supported"));
    }

    validate_header(&doc.header, &mut report);

    if doc.note_count() == 0 {
        report
            .issues
            .push(ValidationIssue::error("song has no notes"));
    }

    let mut all_notes: Vec<(u32, u32, usize, usize)> = Vec::new();
    let mut previous_break: Option<u32> = None;

    for (phrase_index, phrase) in doc.phrases.iter().enumerate() {
        let mut previous_start: Option<u32> = None;

        for (note_index, note) in phrase.notes.iter().enumerate() {
            if note.length_beats == 0 {
                report.issues.push(ValidationIssue::note_error(
                    phrase_index,
                    note_index,
                    "note length must be at least 1 beat",
                ));
            }
            if note.pitch < PITCH_MIN || note.pitch > PITCH_MAX {
                report.issues.push(ValidationIssue::note_error(
                    phrase_index,
                    note_index,
                    format!(
                        "pitch {} is outside the representable range {}..={}",
                        note.pitch, PITCH_MIN, PITCH_MAX
                    ),
                ));
            }
            if !note.is_continuation() && note.text.trim().is_empty() {
                report.issues.push(ValidationIssue::note_error(
                    phrase_index,
                    note_index,
                    "note text is empty",
                ));
            }
            if let Some(previous) = previous_start {
                if note.start_beat < previous {
                    report.issues.push(ValidationIssue::note_error(
                        phrase_index,
                        note_index,
                        "notes are not in chronological order",
                    ));
                }
            }
            previous_start = Some(note.start_beat);

            let end = note.start_beat.saturating_add(note.length_beats.max(1));
            all_notes.push((note.start_beat, end, phrase_index, note_index));
        }

        if let Some(break_marker) = &phrase.break_after {
            if let Some(end) = break_marker.end_beat {
                if end < break_marker.start_beat {
                    report.issues.push(ValidationIssue {
                        severity: Severity::Error,
                        phrase: Some(phrase_index),
                        note: None,
                        message: "phrase break ends before it starts".into(),
                    });
                }
            }
            if let Some(last_note_end) = phrase.end_beat() {
                if break_marker.start_beat < last_note_end {
                    report.issues.push(ValidationIssue {
                        severity: Severity::Error,
                        phrase: Some(phrase_index),
                        note: None,
                        message: "phrase break starts before the last note ends".into(),
                    });
                }
            }
            if let Some(previous) = previous_break {
                if break_marker.start_beat < previous {
                    report.issues.push(ValidationIssue {
                        severity: Severity::Error,
                        phrase: Some(phrase_index),
                        note: None,
                        message: "phrase breaks are not in chronological order".into(),
                    });
                }
            }
            previous_break = Some(break_marker.start_beat);
        }
    }

    all_notes.sort_unstable_by_key(|(start, end, _, _)| (*start, *end));
    let mut active: Option<(u32, usize, usize)> = None;
    for (start, end, phrase_index, note_index) in all_notes {
        if let Some((active_end, active_phrase, active_note)) = active {
            if start < active_end {
                report.issues.push(ValidationIssue::note_error(
                    phrase_index,
                    note_index,
                    format!("note overlaps with phrase {active_phrase}, note {active_note}"),
                ));
            }
            if end > active_end {
                active = Some((end, phrase_index, note_index));
            }
        } else {
            active = Some((end, phrase_index, note_index));
        }
    }

    report
}

fn validate_header(header: &SongHeader, report: &mut ValidationReport) {
    if header.title.trim().is_empty() {
        report
            .issues
            .push(ValidationIssue::error("missing song title"));
    }
    if header.artist.trim().is_empty() {
        report.issues.push(ValidationIssue::error("missing artist"));
    }
    if header.audio.trim().is_empty() {
        report
            .issues
            .push(ValidationIssue::error("missing audio file reference"));
    }
    if !header.bpm.is_finite() || header.bpm <= 0.0 {
        report.issues.push(ValidationIssue::error(
            "BPM must be a positive finite number",
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use usdx_domain::{Note, Phrase, PhraseBreak, SongHeader};

    fn valid_doc() -> SongDoc {
        SongDoc {
            header: SongHeader {
                title: "Song".into(),
                artist: "Artist".into(),
                audio: "audio.mp3".into(),
                bpm: 120.0,
                gap_ms: 1000,
                ..SongHeader::default()
            },
            phrases: vec![
                Phrase {
                    notes: vec![
                        Note {
                            start_beat: 0,
                            length_beats: 2,
                            pitch: 0,
                            text: "Hel".into(),
                            ..Note::default()
                        },
                        Note {
                            start_beat: 2,
                            length_beats: 2,
                            pitch: 2,
                            text: "lo".into(),
                            starts_word: true,
                            ..Note::default()
                        },
                    ],
                    break_after: Some(PhraseBreak {
                        start_beat: 4,
                        end_beat: None,
                    }),
                },
                Phrase {
                    notes: vec![Note {
                        start_beat: 6,
                        length_beats: 1,
                        pitch: 4,
                        text: "~".into(),
                        ..Note::default()
                    }],
                    break_after: None,
                },
            ],
            has_end_marker: true,
            is_duet: false,
        }
    }

    #[test]
    fn valid_song_has_no_errors() {
        let report = validate(&valid_doc());
        assert!(report.is_valid(), "{:?}", report.issues);
        assert!(report.issues.is_empty(), "{:?}", report.issues);
    }

    #[test]
    fn missing_mandatory_headers_are_errors() {
        let mut doc = valid_doc();
        doc.header.title.clear();
        doc.header.artist.clear();
        doc.header.audio.clear();
        doc.header.bpm = 0.0;
        let report = validate(&doc);
        assert_eq!(report.errors().count(), 4);
    }

    #[test]
    fn zero_length_pitch_range_and_empty_text() {
        let mut doc = valid_doc();
        doc.phrases[0].notes[0].length_beats = 0;
        doc.phrases[0].notes[1].pitch = 99;
        doc.phrases[1].notes[0].text = "  ".into();
        let report = validate(&doc);
        assert!(report.issues.iter().any(|i| i.message.contains("length")));
        assert!(report.issues.iter().any(|i| i.message.contains("pitch")));
        assert!(report
            .issues
            .iter()
            .any(|i| i.message.contains("text is empty")));
    }

    #[test]
    fn overlapping_notes_in_one_phrase_are_errors() {
        let mut doc = valid_doc();
        doc.phrases[0].notes[1].start_beat = 1;
        let report = validate(&doc);
        assert!(report.issues.iter().any(|i| i.message.contains("overlaps")));
    }

    #[test]
    fn overlapping_notes_across_phrases_are_errors() {
        let mut doc = valid_doc();
        doc.phrases[1].notes[0].start_beat = 3;
        let report = validate(&doc);
        assert!(report.issues.iter().any(|i| i.message.contains("overlaps")));
    }

    #[test]
    fn out_of_order_notes_are_errors() {
        let mut doc = valid_doc();
        doc.phrases[0].notes[0].start_beat = 5;
        let report = validate(&doc);
        assert!(report
            .issues
            .iter()
            .any(|i| i.message.contains("chronological")));
    }

    #[test]
    fn phrase_break_must_follow_the_last_note_and_be_ordered() {
        let mut doc = valid_doc();
        doc.phrases[0].break_after = Some(PhraseBreak {
            start_beat: 1,
            end_beat: None,
        });
        let report = validate(&doc);
        assert!(report
            .issues
            .iter()
            .any(|i| i.message.contains("before the last note ends")));

        doc.phrases[0].break_after = Some(PhraseBreak {
            start_beat: 10,
            end_beat: Some(9),
        });
        let report = validate(&doc);
        assert!(report
            .issues
            .iter()
            .any(|i| i.message.contains("ends before it starts")));
    }

    #[test]
    fn duet_and_empty_song_are_errors() {
        let mut doc = valid_doc();
        doc.is_duet = true;
        doc.phrases.clear();
        let report = validate(&doc);
        assert!(report.issues.iter().any(|i| i.message.contains("duet")));
        assert!(report.issues.iter().any(|i| i.message.contains("no notes")));
    }

    #[test]
    fn issues_point_to_the_offending_note() {
        let mut doc = valid_doc();
        doc.phrases[1].notes[0].pitch = 99;
        let report = validate(&doc);
        let issue = report
            .issues
            .iter()
            .find(|i| i.message.contains("pitch"))
            .expect("pitch issue");
        assert_eq!(issue.phrase, Some(1));
        assert_eq!(issue.note, Some(0));
    }
}

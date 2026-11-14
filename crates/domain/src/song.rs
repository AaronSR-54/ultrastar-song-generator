use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// A position or length measured in UltraStar beats.
pub type Beat = u32;

/// Pitch expressed as semitones relative to MIDI 60 (C4), matching the `.txt` format.
pub type Pitch = i16;

/// Note type marker used by UltraStar.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NoteKind {
    /// `:` normal note.
    #[default]
    Normal,
    /// `*` golden note.
    Golden,
    /// `F` freestyle note.
    Freestyle,
    /// `R` rap note.
    Rap,
    /// `G` golden rap note.
    RapGolden,
}

/// Metadata block at the top of an UltraStar file.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct SongHeader {
    pub title: String,
    pub artist: String,
    /// Value of `#MP3`, relative to the song folder.
    pub audio: String,
    /// UltraStar BPM, i.e. four times the musical tempo.
    pub bpm: f64,
    /// Milliseconds added to every note timestamp.
    pub gap_ms: i64,
    pub language: Option<String>,
    pub genre: Option<String>,
    pub year: Option<i32>,
    pub creator: Option<String>,
    pub cover: Option<String>,
    pub video: Option<String>,
    pub video_gap_s: Option<f64>,
    pub background: Option<String>,
    pub edition: Option<String>,
    pub medley_start_beat: Option<Beat>,
    pub medley_end_beat: Option<Beat>,
    /// Unknown headers preserved verbatim for a faithful round trip.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub extra: BTreeMap<String, String>,
}

impl Default for SongHeader {
    fn default() -> Self {
        Self {
            title: String::new(),
            artist: String::new(),
            audio: String::new(),
            bpm: 120.0,
            gap_ms: 0,
            language: None,
            genre: None,
            year: None,
            creator: None,
            cover: None,
            video: None,
            video_gap_s: None,
            background: None,
            edition: None,
            medley_start_beat: None,
            medley_end_beat: None,
            extra: BTreeMap::new(),
        }
    }
}

impl SongHeader {
    /// Converts a musical tempo into the quadrupled value stored in `#BPM`.
    pub fn bpm_from_tempo(tempo_bpm: f64) -> f64 {
        tempo_bpm * 4.0
    }

    /// Duration of one UltraStar beat in milliseconds.
    pub fn beat_duration_ms(&self) -> f64 {
        if self.bpm <= 0.0 {
            return 0.0;
        }
        60_000.0 / (self.bpm * 4.0)
    }
}

/// A single singable note.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Note {
    pub kind: NoteKind,
    pub start_beat: Beat,
    pub length_beats: Beat,
    pub pitch: Pitch,
    /// Syllable text without the leading word separator space.
    pub text: String,
    /// True when the note starts a new word and the writer must emit a leading space.
    pub starts_word: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub confidence: Option<f32>,
}

impl Default for Note {
    fn default() -> Self {
        Self {
            kind: NoteKind::Normal,
            start_beat: 0,
            length_beats: 1,
            pitch: 0,
            text: String::new(),
            starts_word: false,
            confidence: None,
        }
    }
}

impl Note {
    /// A continuation note carries `~` instead of new lyrics.
    pub fn is_continuation(&self) -> bool {
        self.text.trim() == "~"
    }

    pub fn end_beat(&self) -> Beat {
        self.start_beat.saturating_add(self.length_beats)
    }
}

/// Trailing `-` marker that closes a phrase.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PhraseBreak {
    pub start_beat: Beat,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end_beat: Option<Beat>,
}

/// A group of notes followed by an optional `-` break.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Phrase {
    #[serde(default)]
    pub notes: Vec<Note>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub break_after: Option<PhraseBreak>,
}

impl Phrase {
    pub fn start_beat(&self) -> Option<Beat> {
        self.notes.first().map(|note| note.start_beat)
    }

    pub fn end_beat(&self) -> Option<Beat> {
        self.notes.last().map(Note::end_beat)
    }
}

/// Complete song document in the domain model.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SongDoc {
    pub header: SongHeader,
    #[serde(default)]
    pub phrases: Vec<Phrase>,
    #[serde(default)]
    pub has_end_marker: bool,
    /// True when the source file declared `P1`/`P2` duet sections.
    #[serde(default)]
    pub is_duet: bool,
}

impl SongDoc {
    pub fn notes(&self) -> impl Iterator<Item = &Note> {
        self.phrases.iter().flat_map(|phrase| phrase.notes.iter())
    }

    pub fn note_count(&self) -> usize {
        self.phrases.iter().map(|phrase| phrase.notes.len()).sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn header_defaults_are_valid() {
        let header = SongHeader::default();
        assert_eq!(header.title, "");
        assert_eq!(header.artist, "");
        assert_eq!(header.bpm, 120.0);
        assert_eq!(header.gap_ms, 0);
        assert!(header.extra.is_empty());
        assert!(header.language.is_none());
    }

    #[test]
    fn note_defaults_have_minimum_length() {
        let note = Note::default();
        assert_eq!(note.kind, NoteKind::Normal);
        assert_eq!(note.start_beat, 0);
        assert_eq!(note.length_beats, 1);
        assert_eq!(note.pitch, 0);
        assert!(!note.is_continuation());
        assert_eq!(note.end_beat(), 1);
    }

    #[test]
    fn continuation_detection_ignores_surrounding_space() {
        let note = Note {
            text: "  ~ ".into(),
            ..Note::default()
        };
        assert!(note.is_continuation());
    }

    #[test]
    fn song_doc_round_trips_through_json() {
        let doc = SongDoc {
            header: SongHeader {
                title: "Canción".into(),
                artist: "Artista".into(),
                audio: "audio.mp3".into(),
                bpm: 313.34,
                gap_ms: 81_090,
                language: Some("Spanish".into()),
                cover: Some("cover.jpg".into()),
                extra: BTreeMap::from([("CUSTOM".into(), "value".into())]),
                ..SongHeader::default()
            },
            phrases: vec![
                Phrase {
                    notes: vec![
                        Note {
                            start_beat: 0,
                            length_beats: 2,
                            pitch: -2,
                            text: "Hel".into(),
                            starts_word: true,
                            ..Note::default()
                        },
                        Note {
                            start_beat: 3,
                            length_beats: 4,
                            pitch: -2,
                            text: "lo".into(),
                            ..Note::default()
                        },
                        Note {
                            start_beat: 8,
                            length_beats: 2,
                            pitch: 0,
                            text: "~".into(),
                            ..Note::default()
                        },
                    ],
                    break_after: Some(PhraseBreak {
                        start_beat: 10,
                        end_beat: Some(12),
                    }),
                },
                Phrase {
                    notes: vec![Note {
                        kind: NoteKind::Golden,
                        start_beat: 12,
                        length_beats: 3,
                        pitch: 4,
                        text: "gold".into(),
                        starts_word: true,
                        confidence: Some(0.9),
                    }],
                    break_after: None,
                },
            ],
            has_end_marker: true,
            ..SongDoc::default()
        };

        let json = serde_json::to_string(&doc).expect("serialize");
        let parsed: SongDoc = serde_json::from_str(&json).expect("deserialize");

        assert_eq!(parsed, doc);
        assert_eq!(parsed.note_count(), 4);
        assert_eq!(parsed.notes().count(), 4);
    }

    #[test]
    fn tempo_is_quadrupled_for_usdx() {
        assert!((SongHeader::bpm_from_tempo(78.335) - 313.34).abs() < 1e-9);
        let header = SongHeader {
            bpm: 313.34,
            ..SongHeader::default()
        };
        assert!((header.beat_duration_ms() - 47.873).abs() < 0.01);
    }
}

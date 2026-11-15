//! Editable song state for the review editor.
//!
//! Every mutation goes through this type, which keeps the note invariants
//! (temporal order, minimum length, no overlaps), records undo/redo snapshots
//! and tracks autosave. The GUI only renders it and calls these methods, so all
//! edit rules are covered by headless tests.

use std::time::Instant;

use usdx_application::Autosave;
use usdx_domain::{Beat, Note, NoteKind, Phrase, SongDoc, SongHeader, PITCH_MAX, PITCH_MIN};
use usdx_format::{Severity, ValidationReport};

/// Number of undo snapshots kept in memory.
pub const UNDO_LIMIT: usize = 100;
/// Autosave delay after the last edit.
pub const AUTOSAVE_DELAY: std::time::Duration = std::time::Duration::from_secs(2);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NoteSelection {
    pub phrase: usize,
    pub note: usize,
}

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum EditorError {
    #[error("no note is selected")]
    NoSelection,
    #[error("phrase {0} does not exist")]
    PhraseOutOfRange(usize),
    #[error("phrase {phrase} has no note {note}")]
    NoteOutOfRange { phrase: usize, note: usize },
    #[error("the new note would overlap a neighbouring note")]
    Overlap,
    #[error("BPM must be greater than zero and at most {0}")]
    InvalidBpm(f64),
    #[error("GAP must be between -600000 and 600000 ms")]
    InvalidGap(i64),
}

/// Result of an export attempt from the editor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExportCheck {
    Ready,
    /// The editor selected the offending note and reports why.
    Blocked {
        selection: Option<NoteSelection>,
        message: String,
    },
}

#[derive(Debug)]
pub struct EditorState {
    document: SongDoc,
    selection: Option<NoteSelection>,
    undo: Vec<SongDoc>,
    redo: Vec<SongDoc>,
    dirty: bool,
    autosave: Autosave,
    pub last_message: Option<String>,
}

impl EditorState {
    pub fn new(document: SongDoc) -> Self {
        let selection = document.phrases.first().and_then(|phrase| {
            (!phrase.notes.is_empty()).then_some(NoteSelection { phrase: 0, note: 0 })
        });
        Self {
            document,
            selection,
            undo: Vec::new(),
            redo: Vec::new(),
            dirty: false,
            autosave: Autosave::new(AUTOSAVE_DELAY),
            last_message: None,
        }
    }

    pub fn document(&self) -> &SongDoc {
        &self.document
    }

    pub fn header(&self) -> &SongHeader {
        &self.document.header
    }

    pub fn selection(&self) -> Option<NoteSelection> {
        self.selection
    }

    pub fn selected_note(&self) -> Option<&Note> {
        let selection = self.selection?;
        self.document
            .phrases
            .get(selection.phrase)?
            .notes
            .get(selection.note)
    }

    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    pub fn undo_depth(&self) -> usize {
        self.undo.len()
    }

    pub fn autosave(&mut self) -> &mut Autosave {
        &mut self.autosave
    }

    pub fn mark_saved(&mut self) {
        self.dirty = false;
        self.autosave.mark_saved();
    }

    pub fn should_autosave(&self, now: Instant) -> bool {
        self.autosave.should_save(now)
    }

    pub fn select(&mut self, phrase: usize, note: usize) -> Result<(), EditorError> {
        let phrase_ref = self
            .document
            .phrases
            .get(phrase)
            .ok_or(EditorError::PhraseOutOfRange(phrase))?;
        if note >= phrase_ref.notes.len() {
            return Err(EditorError::NoteOutOfRange { phrase, note });
        }
        self.selection = Some(NoteSelection { phrase, note });
        Ok(())
    }

    pub fn clear_selection(&mut self) {
        self.selection = None;
    }

    /// Moves the selection forward or backward through all notes.
    pub fn select_relative(&mut self, forward: bool) {
        let flattened: Vec<NoteSelection> = self
            .document
            .phrases
            .iter()
            .enumerate()
            .flat_map(|(phrase, value)| {
                (0..value.notes.len()).map(move |note| NoteSelection { phrase, note })
            })
            .collect();
        if flattened.is_empty() {
            self.selection = None;
            return;
        }
        let current = self
            .selection
            .and_then(|selection| flattened.iter().position(|value| *value == selection));
        let index = match (current, forward) {
            (Some(index), true) => (index + 1).min(flattened.len() - 1),
            (Some(index), false) => index.saturating_sub(1),
            (None, _) => 0,
        };
        self.selection = flattened.get(index).copied();
    }

    fn snapshot(&mut self) {
        self.undo.push(self.document.clone());
        if self.undo.len() > UNDO_LIMIT {
            self.undo.remove(0);
        }
        self.redo.clear();
    }

    fn touched(&mut self, now: Instant) {
        self.dirty = true;
        self.autosave.mark_dirty(now);
    }

    pub fn undo(&mut self) -> bool {
        let Some(previous) = self.undo.pop() else {
            return false;
        };
        self.redo
            .push(std::mem::replace(&mut self.document, previous));
        self.sanitize_selection();
        self.dirty = true;
        self.autosave.mark_dirty(Instant::now());
        true
    }

    pub fn redo(&mut self) -> bool {
        let Some(next) = self.redo.pop() else {
            return false;
        };
        self.undo.push(std::mem::replace(&mut self.document, next));
        self.sanitize_selection();
        self.dirty = true;
        self.autosave.mark_dirty(Instant::now());
        true
    }

    fn sanitize_selection(&mut self) {
        if let Some(selection) = self.selection {
            let valid = self
                .document
                .phrases
                .get(selection.phrase)
                .map(|phrase| selection.note < phrase.notes.len())
                .unwrap_or(false);
            if !valid {
                self.selection = self
                    .document
                    .phrases
                    .first()
                    .filter(|phrase| !phrase.notes.is_empty())
                    .map(|_| NoteSelection { phrase: 0, note: 0 });
            }
        }
    }

    /// Inserts a note only where it fits without moving anything.
    pub fn add_note(
        &mut self,
        phrase: usize,
        start_beat: Beat,
        length_beats: Beat,
        pitch: i16,
        now: Instant,
    ) -> Result<NoteSelection, EditorError> {
        let target = self
            .document
            .phrases
            .get(phrase)
            .ok_or(EditorError::PhraseOutOfRange(phrase))?;
        let length = length_beats.max(1);
        let end = start_beat.saturating_add(length);
        for existing in &target.notes {
            let existing_end = existing.start_beat + existing.length_beats;
            if start_beat < existing_end && existing.start_beat < end {
                return Err(EditorError::Overlap);
            }
        }

        self.snapshot();
        let note = Note {
            kind: NoteKind::Normal,
            start_beat,
            length_beats: length,
            pitch: pitch.clamp(PITCH_MIN, PITCH_MAX),
            text: "".into(),
            starts_word: false,
            confidence: None,
        };
        let target = &mut self.document.phrases[phrase];
        target.notes.push(note);
        target.notes.sort_by_key(|note| note.start_beat);
        let index = target
            .notes
            .iter()
            .position(|candidate| candidate.start_beat == start_beat)
            .unwrap_or(0);
        self.selection = Some(NoteSelection {
            phrase,
            note: index,
        });
        self.touched(now);
        Ok(NoteSelection {
            phrase,
            note: index,
        })
    }

    /// Removes the selected note. When the note opened a word, the syllable that
    /// takes its place inherits the word opening, so the printed lyrics keep
    /// their space; deleting the last note of a phrase removes the phrase.
    pub fn delete_selected(&mut self, now: Instant) -> Result<(), EditorError> {
        let Some(selection) = self.selection else {
            return Ok(());
        };
        let removed = self
            .document
            .phrases
            .get(selection.phrase)
            .ok_or(EditorError::PhraseOutOfRange(selection.phrase))?
            .notes
            .get(selection.note)
            .ok_or(EditorError::NoteOutOfRange {
                phrase: selection.phrase,
                note: selection.note,
            })?
            .clone();

        self.snapshot();
        let emptied = {
            let phrase = &mut self.document.phrases[selection.phrase];
            phrase.notes.remove(selection.note);
            if removed.starts_word {
                if let Some(next) = phrase.notes.get_mut(selection.note) {
                    if !next.is_continuation() {
                        next.starts_word = true;
                    }
                }
            }
            phrase.notes.is_empty()
        };
        if emptied {
            self.document.phrases.remove(selection.phrase);
        }

        // Keep the cursor where the note was, so deleting a run is one key press
        // each instead of jumping back to the start of the song.
        if emptied {
            self.selection = self
                .document
                .phrases
                .get(selection.phrase)
                .map(|_| NoteSelection {
                    phrase: selection.phrase,
                    note: 0,
                })
                .or_else(|| {
                    let phrase = self.document.phrases.len().checked_sub(1)?;
                    let note = self.document.phrases[phrase].notes.len().checked_sub(1)?;
                    Some(NoteSelection { phrase, note })
                });
        } else if let Some(phrase) = self.document.phrases.get(selection.phrase) {
            let last = phrase.notes.len().saturating_sub(1);
            self.selection = Some(NoteSelection {
                phrase: selection.phrase,
                note: selection.note.min(last),
            });
        }
        self.sanitize_selection();
        self.touched(now);
        Ok(())
    }

    /// Applies a note type to the selection, or returns it to normal when it
    /// already has that type. Freestyle and rap keep the syllable text.
    pub fn toggle_kind(&mut self, kind: NoteKind, now: Instant) -> Result<NoteKind, EditorError> {
        let Some(selection) = self.selection else {
            return Err(EditorError::NoSelection);
        };
        let current = self
            .selected_note()
            .ok_or(EditorError::NoteOutOfRange {
                phrase: selection.phrase,
                note: selection.note,
            })?
            .kind;
        let next = if current == kind {
            NoteKind::Normal
        } else {
            kind
        };
        self.snapshot();
        self.document.phrases[selection.phrase].notes[selection.note].kind = next;
        self.touched(now);
        Ok(next)
    }

    /// Moves a note horizontally, clamped so it never invades its neighbours.
    pub fn move_note(
        &mut self,
        phrase: usize,
        note: usize,
        start_beat: Beat,
        now: Instant,
    ) -> Result<Beat, EditorError> {
        let phrase_ref = self
            .document
            .phrases
            .get(phrase)
            .ok_or(EditorError::PhraseOutOfRange(phrase))?;
        let target = phrase_ref
            .notes
            .get(note)
            .ok_or(EditorError::NoteOutOfRange { phrase, note })?;
        let (lower, upper) = neighbour_bounds(phrase_ref, note);
        let length = target.length_beats;
        let mut start = start_beat.max(lower);
        if let Some(upper) = upper {
            start = start.min(upper.saturating_sub(length));
        }
        start = start.max(lower);

        self.snapshot();
        self.document.phrases[phrase].notes[note].start_beat = start;
        self.touched(now);
        Ok(start)
    }

    /// Resizes a note, keeping at least one beat and stopping at the next note.
    pub fn resize_note(
        &mut self,
        phrase: usize,
        note: usize,
        length_beats: Beat,
        now: Instant,
    ) -> Result<Beat, EditorError> {
        let phrase_ref = self
            .document
            .phrases
            .get(phrase)
            .ok_or(EditorError::PhraseOutOfRange(phrase))?;
        let target = phrase_ref
            .notes
            .get(note)
            .ok_or(EditorError::NoteOutOfRange { phrase, note })?;
        let (_, upper) = neighbour_bounds(phrase_ref, note);
        let maximum = upper
            .map(|upper| upper.saturating_sub(target.start_beat).max(1))
            .unwrap_or(u32::MAX);
        let length = length_beats.clamp(1, maximum);

        self.snapshot();
        self.document.phrases[phrase].notes[note].length_beats = length;
        self.touched(now);
        Ok(length)
    }

    pub fn bump_pitch(&mut self, delta: i16, now: Instant) -> Result<i16, EditorError> {
        let Some(selection) = self.selection else {
            return Err(EditorError::NoSelection);
        };
        let note = self
            .document
            .phrases
            .get(selection.phrase)
            .and_then(|phrase| phrase.notes.get(selection.note))
            .ok_or(EditorError::NoteOutOfRange {
                phrase: selection.phrase,
                note: selection.note,
            })?;
        let pitch = (note.pitch + delta).clamp(PITCH_MIN, PITCH_MAX);
        self.snapshot();
        self.document.phrases[selection.phrase].notes[selection.note].pitch = pitch;
        self.touched(now);
        Ok(pitch)
    }

    pub fn set_pitch(&mut self, pitch: i16, now: Instant) -> Result<i16, EditorError> {
        let Some(selection) = self.selection else {
            return Err(EditorError::NoSelection);
        };
        self.note_mut(selection)?;
        let pitch = pitch.clamp(PITCH_MIN, PITCH_MAX);
        self.snapshot();
        self.document.phrases[selection.phrase].notes[selection.note].pitch = pitch;
        self.touched(now);
        Ok(pitch)
    }

    pub fn set_kind(&mut self, kind: NoteKind, now: Instant) -> Result<(), EditorError> {
        let Some(selection) = self.selection else {
            return Err(EditorError::NoSelection);
        };
        self.note_mut(selection)?;
        self.snapshot();
        self.document.phrases[selection.phrase].notes[selection.note].kind = kind;
        self.touched(now);
        Ok(())
    }

    pub fn set_text(&mut self, text: impl Into<String>, now: Instant) -> Result<(), EditorError> {
        let Some(selection) = self.selection else {
            return Err(EditorError::NoSelection);
        };
        self.note_mut(selection)?;
        self.snapshot();
        let note = &mut self.document.phrases[selection.phrase].notes[selection.note];
        note.text = text.into();
        note.confidence = None;
        self.touched(now);
        Ok(())
    }

    /// Converts the selected note into a `~` continuation, or back to text.
    pub fn toggle_continuation(&mut self, now: Instant) -> Result<(), EditorError> {
        let Some(selection) = self.selection else {
            return Err(EditorError::NoSelection);
        };
        let is_continuation = self.note_mut(selection)?.is_continuation();
        self.snapshot();
        let note = &mut self.document.phrases[selection.phrase].notes[selection.note];
        if is_continuation {
            note.text = "la".into();
            note.starts_word = false;
        } else {
            note.text = "~".into();
            note.starts_word = false;
        }
        note.confidence = None;
        self.touched(now);
        Ok(())
    }

    fn note_mut(&mut self, selection: NoteSelection) -> Result<&mut Note, EditorError> {
        self.document
            .phrases
            .get_mut(selection.phrase)
            .ok_or(EditorError::PhraseOutOfRange(selection.phrase))?
            .notes
            .get_mut(selection.note)
            .ok_or(EditorError::NoteOutOfRange {
                phrase: selection.phrase,
                note: selection.note,
            })
    }

    /// Applies a new BPM without moving the audio; only the visual grid changes.
    pub fn set_bpm(&mut self, bpm: f64, now: Instant) -> Result<(), EditorError> {
        if !bpm.is_finite() || bpm <= 0.0 || bpm > 2000.0 {
            return Err(EditorError::InvalidBpm(bpm));
        }
        self.snapshot();
        self.document.header.bpm = bpm;
        self.touched(now);
        Ok(())
    }

    pub fn set_gap_ms(&mut self, gap_ms: i64, now: Instant) -> Result<(), EditorError> {
        if !(-600_000..=600_000).contains(&gap_ms) {
            return Err(EditorError::InvalidGap(gap_ms));
        }
        self.snapshot();
        self.document.header.gap_ms = gap_ms;
        self.touched(now);
        Ok(())
    }

    pub fn apply_tempo_candidate(
        &mut self,
        bpm_usdx: f64,
        gap_ms: i64,
        now: Instant,
    ) -> Result<(), EditorError> {
        if !bpm_usdx.is_finite() || bpm_usdx <= 0.0 || bpm_usdx > 2000.0 {
            return Err(EditorError::InvalidBpm(bpm_usdx));
        }
        if !(-600_000..=600_000).contains(&gap_ms) {
            return Err(EditorError::InvalidGap(gap_ms));
        }
        self.snapshot();
        self.document.header.bpm = bpm_usdx;
        self.document.header.gap_ms = gap_ms;
        self.touched(now);
        Ok(())
    }

    /// Checks the document and, when it cannot be exported, selects the note
    /// that must be fixed.
    pub fn check_export(&mut self) -> ExportCheck {
        let report = usdx_format::validate(&self.document);
        let Some(issue) = report
            .issues
            .iter()
            .find(|issue| issue.severity == Severity::Error)
        else {
            return ExportCheck::Ready;
        };
        let selection = match (issue.phrase, issue.note) {
            (Some(phrase), Some(note)) => {
                let _ = self.select(phrase, note);
                Some(NoteSelection { phrase, note })
            }
            _ => None,
        };
        self.last_message = Some(issue.to_string());
        ExportCheck::Blocked {
            selection,
            message: issue.to_string(),
        }
    }

    /// Applies the error returned by the pipeline export, selecting a note if
    /// the validation failed.
    pub fn adopt_export_failure(&mut self, message: impl Into<String>) -> ExportCheck {
        let message = message.into();
        self.last_message = Some(message);
        self.check_export()
    }

    pub fn report(&self) -> ValidationReport {
        usdx_format::validate(&self.document)
    }
}

fn neighbour_bounds(phrase: &Phrase, index: usize) -> (Beat, Option<Beat>) {
    let lower = if index == 0 {
        0
    } else {
        let previous = &phrase.notes[index - 1];
        previous.start_beat + previous.length_beats
    };
    let upper = phrase.notes.get(index + 1).map(|next| next.start_beat);
    (lower, upper)
}

#[cfg(test)]
mod tests {
    use super::*;
    use usdx_domain::SongHeader;

    fn note(start_beat: Beat, length_beats: Beat, pitch: i16, text: &str) -> Note {
        Note {
            kind: NoteKind::Normal,
            start_beat,
            length_beats,
            pitch,
            text: text.into(),
            starts_word: true,
            confidence: Some(0.9),
        }
    }

    fn document() -> SongDoc {
        SongDoc {
            header: SongHeader {
                title: "Luna".into(),
                artist: "Artista".into(),
                audio: "audio.mp3".into(),
                bpm: 120.0,
                gap_ms: 0,
                ..SongHeader::default()
            },
            phrases: vec![
                Phrase {
                    notes: vec![note(0, 2, 0, "ho"), note(2, 2, 2, "la")],
                    break_after: None,
                },
                Phrase {
                    notes: vec![note(8, 2, -1, "sol")],
                    break_after: None,
                },
            ],
            has_end_marker: true,
            is_duet: false,
        }
    }

    fn editor() -> EditorState {
        EditorState::new(document())
    }

    #[test]
    fn adding_a_note_in_a_gap_keeps_the_order() {
        let mut state = editor();
        let selection = state
            .add_note(0, 4, 2, 4, Instant::now())
            .expect("gap after la");
        assert_eq!(selection, NoteSelection { phrase: 0, note: 2 });
        assert_eq!(state.document().note_count(), 4);
        assert_eq!(state.document().phrases[0].notes[2].start_beat, 4);
        assert!(state.is_dirty());
    }

    #[test]
    fn adding_an_overlapping_note_is_rejected() {
        let mut state = editor();
        let error = state.add_note(0, 1, 2, 4, Instant::now()).unwrap_err();
        assert_eq!(error, EditorError::Overlap);
        assert_eq!(state.document().note_count(), 3);
        assert!(!state.is_dirty());
    }

    #[test]
    fn moving_and_resizing_never_invade_neighbours() {
        let mut state = editor();
        // Moving left is clamped to the previous note's end.
        let start = state.move_note(0, 1, 0, Instant::now()).unwrap();
        assert_eq!(start, 2);
        // Moving right is clamped so the length still fits before... nothing.
        let start = state.move_note(0, 1, 20, Instant::now()).unwrap();
        assert_eq!(start, 20);
        // A resize cannot pass the next note.
        let length = state.resize_note(0, 0, 30, Instant::now()).unwrap();
        assert_eq!(length, 20);
        // And never goes below one beat.
        let length = state.resize_note(0, 0, 0, Instant::now()).unwrap();
        assert_eq!(length, 1);
    }

    #[test]
    fn pitch_kind_and_text_changes_are_applied() {
        let mut state = editor();
        state.select(0, 0).unwrap();
        assert_eq!(state.bump_pitch(1, Instant::now()).unwrap(), 1);
        assert_eq!(state.bump_pitch(-100, Instant::now()).unwrap(), PITCH_MIN);
        state.set_pitch(PITCH_MAX + 10, Instant::now()).unwrap();
        assert_eq!(state.selected_note().unwrap().pitch, PITCH_MAX);

        state.set_kind(NoteKind::Golden, Instant::now()).unwrap();
        assert_eq!(state.selected_note().unwrap().kind, NoteKind::Golden);

        state.set_text("nueva", Instant::now()).unwrap();
        assert_eq!(state.selected_note().unwrap().text, "nueva");
        assert!(state.selected_note().unwrap().confidence.is_none());
    }

    #[test]
    fn continuation_round_trip() {
        let mut state = editor();
        state.select(0, 0).unwrap();
        state.toggle_continuation(Instant::now()).unwrap();
        assert!(state.selected_note().unwrap().is_continuation());
        state.toggle_continuation(Instant::now()).unwrap();
        assert!(!state.selected_note().unwrap().is_continuation());
        assert!(!state.selected_note().unwrap().text.is_empty());
    }

    #[test]
    fn deleting_the_last_note_removes_the_phrase() {
        let mut state = editor();
        state.select(1, 0).unwrap();
        state.delete_selected(Instant::now()).unwrap();
        assert_eq!(state.document().phrases.len(), 1);
        assert_eq!(state.document().note_count(), 2);
    }

    #[test]
    fn deleting_a_word_opening_hands_the_word_to_the_next_syllable() {
        let mut state = EditorState::new(SongDoc {
            header: SongHeader::default(),
            phrases: vec![Phrase {
                notes: vec![
                    Note {
                        text: "can".into(),
                        starts_word: true,
                        ..note(0, 2, 0, "can")
                    },
                    Note {
                        text: "ción".into(),
                        starts_word: false,
                        ..note(2, 2, 2, "ción")
                    },
                    Note {
                        text: "ya".into(),
                        starts_word: true,
                        ..note(4, 2, 0, "ya")
                    },
                ],
                break_after: None,
            }],
            has_end_marker: true,
            ..SongDoc::default()
        });

        state.select(0, 0).unwrap();
        state.delete_selected(Instant::now()).unwrap();

        let phrase = &state.document().phrases[0];
        assert_eq!(phrase.notes.len(), 2);
        // "ción" now opens the word, so the writer still emits its leading space.
        assert!(phrase.notes[0].starts_word);
        assert_eq!(phrase.notes[0].text, "ción");
        assert!(phrase.notes[1].starts_word);
    }

    #[test]
    fn deleting_keeps_the_cursor_next_to_the_gap() {
        let mut state = editor();
        state.select(0, 0).unwrap();
        state.delete_selected(Instant::now()).unwrap();
        assert_eq!(
            state.selection(),
            Some(NoteSelection { phrase: 0, note: 0 })
        );
        assert_eq!(state.document().phrases[0].notes[0].text, "la");

        // Deleting the only note of the second phrase lands on the last note
        // that is left, which is the survivor of the first delete.
        state.select(1, 0).unwrap();
        state.delete_selected(Instant::now()).unwrap();
        assert_eq!(
            state.selection(),
            Some(NoteSelection { phrase: 0, note: 0 })
        );
        assert_eq!(state.document().phrases.len(), 1);
    }

    #[test]
    fn kind_toggles_return_to_normal() {
        let mut state = editor();
        state.select(0, 0).unwrap();
        assert_eq!(
            state
                .toggle_kind(NoteKind::Freestyle, Instant::now())
                .unwrap(),
            NoteKind::Freestyle
        );
        assert!(!state.selected_note().unwrap().is_continuation());
        // The text survives: the format does not allow an empty note.
        assert_eq!(state.selected_note().unwrap().text, "ho");
        assert_eq!(
            state
                .toggle_kind(NoteKind::Freestyle, Instant::now())
                .unwrap(),
            NoteKind::Normal
        );
        assert_eq!(
            state.toggle_kind(NoteKind::Rap, Instant::now()).unwrap(),
            NoteKind::Rap
        );
    }

    #[test]
    fn undo_and_redo_restore_exact_beats() {
        let mut state = editor();
        state.move_note(0, 1, 10, Instant::now()).unwrap();
        assert_eq!(state.document().phrases[0].notes[1].start_beat, 10);
        assert!(state.can_undo());

        assert!(state.undo());
        assert_eq!(state.document().phrases[0].notes[1].start_beat, 2);
        assert!(state.can_redo());

        assert!(state.redo());
        assert_eq!(state.document().phrases[0].notes[1].start_beat, 10);
        // Redo is applied exactly once: one more redo is a no-op.
        assert!(!state.redo());
    }

    #[test]
    fn undo_history_is_limited_to_one_hundred_snapshots() {
        let mut state = editor();
        for pitch in 0..130_i16 {
            state.select(0, 0).unwrap();
            state.set_pitch(pitch % 20, Instant::now()).unwrap();
        }
        assert_eq!(state.undo_depth(), UNDO_LIMIT);
        let mut undone = 0;
        while state.undo() {
            undone += 1;
        }
        assert_eq!(undone, UNDO_LIMIT);
    }

    #[test]
    fn bpm_and_gap_reject_invalid_values_without_changing_the_document() {
        let mut state = editor();
        assert_eq!(
            state.set_bpm(0.0, Instant::now()),
            Err(EditorError::InvalidBpm(0.0))
        );
        assert!(state.set_bpm(-4.0, Instant::now()).is_err());
        assert!(state.set_bpm(f64::NAN, Instant::now()).is_err());
        assert_eq!(state.document().header.bpm, 120.0);
        assert!(!state.can_undo());

        state.set_bpm(313.34, Instant::now()).unwrap();
        assert_eq!(state.document().header.bpm, 313.34);
        state.set_gap_ms(-2_500, Instant::now()).unwrap();
        assert_eq!(state.document().header.gap_ms, -2_500);
        assert_eq!(
            state.set_gap_ms(1_000_000, Instant::now()),
            Err(EditorError::InvalidGap(1_000_000))
        );
    }

    #[test]
    fn tempo_candidates_update_bpm_and_gap_together() {
        let mut state = editor();
        state
            .apply_tempo_candidate(250.0, 810, Instant::now())
            .unwrap();
        assert_eq!(state.document().header.bpm, 250.0);
        assert_eq!(state.document().header.gap_ms, 810);
        // The audio is not moved: only the grid parameters change.
        assert_eq!(state.document().phrases[0].notes[0].start_beat, 0);
    }

    #[test]
    fn autosave_fires_after_the_delay_and_clears_on_save() {
        let mut state = editor();
        let start = Instant::now();
        state.set_bpm(200.0, start).unwrap();
        assert!(state.is_dirty());
        assert!(!state.should_autosave(start));
        assert!(state.should_autosave(start + AUTOSAVE_DELAY + std::time::Duration::from_millis(1)));
        state.mark_saved();
        assert!(!state.is_dirty());
        assert!(!state.should_autosave(start + AUTOSAVE_DELAY * 2));
    }

    #[test]
    fn export_check_selects_the_note_with_empty_text() {
        let mut document = document();
        document.phrases[1].notes[0].text.clear();
        let mut state = EditorState::new(document);
        let check = state.check_export();
        match check {
            ExportCheck::Blocked { selection, message } => {
                assert_eq!(selection, Some(NoteSelection { phrase: 1, note: 0 }));
                assert!(!message.is_empty());
            }
            ExportCheck::Ready => panic!("an empty note must block export"),
        }
        assert_eq!(
            state.selection(),
            Some(NoteSelection { phrase: 1, note: 0 })
        );
    }

    #[test]
    fn a_valid_document_is_ready_to_export() {
        let mut state = editor();
        assert_eq!(state.check_export(), ExportCheck::Ready);
        assert!(state.report().is_valid());
    }

    #[test]
    fn export_failure_from_the_pipeline_selects_the_note() {
        let mut document = document();
        document.phrases[0].notes[1].text.clear();
        let mut state = EditorState::new(document);
        let check = state.adopt_export_failure("validation failed");
        assert!(matches!(check, ExportCheck::Blocked { .. }));
        assert_eq!(
            state.selection(),
            Some(NoteSelection { phrase: 0, note: 1 })
        );
    }

    #[test]
    fn selection_moves_through_all_notes() {
        let mut state = editor();
        state.select(0, 0).unwrap();
        state.select_relative(true);
        assert_eq!(
            state.selection(),
            Some(NoteSelection { phrase: 0, note: 1 })
        );
        state.select_relative(true);
        assert_eq!(
            state.selection(),
            Some(NoteSelection { phrase: 1, note: 0 })
        );
        state.select_relative(true);
        assert_eq!(
            state.selection(),
            Some(NoteSelection { phrase: 1, note: 0 })
        );
        state.select_relative(false);
        assert_eq!(
            state.selection(),
            Some(NoteSelection { phrase: 0, note: 1 })
        );
    }
}

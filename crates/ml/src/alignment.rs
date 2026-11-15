//! Monotonic dynamic-programming alignment between lyric syllables and the
//! detected notes.
//!
//! Each syllable gets a time anchor interpolated inside its Whisper word, and
//! the DP walks syllables and note onsets in order. Matching costs combine
//! time distance, duration fit and pitch continuity. Skipping a note is cheap
//! (it becomes a `~` continuation), skipping a syllable is expensive, and when
//! a detected note has to carry several syllables it is split so every
//! syllable keeps its own note.
//!
//! The result is a complete [`Alignment`] that can be turned into a
//! [`usdx_domain::SongDoc`] for the writer.

use serde::{Deserialize, Serialize};
use usdx_domain::{
    Note, NoteKind, Phrase, PhraseBreak, SongDoc, SongHeader, Transcript, TranscriptLine,
};
use usdx_format::ms_to_beat;

use crate::quantize::{QuantizedNote, QuantizedSong};
use crate::transcript_edit::Syllabifier;

/// Text used by UltraStar for a note that continues the previous syllable.
pub const CONTINUATION_TEXT: &str = "~";

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AlignmentOptions {
    /// A note below this confidence is flagged for review.
    pub min_confidence: f32,
    /// Time distance in seconds that halves the time score (cost 1.0).
    ///
    /// Seconds, not beats: `#BPM` varies wildly between songs (a beat can last
    /// 40 ms or 250 ms) and a beat-based tolerance would be far stricter on
    /// fast songs.
    pub time_tolerance_s: f64,
    /// Silence in seconds that starts a new phrase.
    pub phrase_gap_s: f64,
    /// Maximum syllables the DP may put on a single note.
    pub max_grouped: usize,
    /// Cost of turning a note into a `~` continuation.
    pub skip_note_cost: f64,
    /// Base cost of grouping syllables on one note.
    pub group_cost: f64,
    /// Additional cost per extra grouped syllable.
    pub group_syllable_cost: f64,
}

impl Default for AlignmentOptions {
    fn default() -> Self {
        Self {
            min_confidence: 0.55,
            time_tolerance_s: 0.3,
            phrase_gap_s: 0.8,
            max_grouped: 4,
            skip_note_cost: 0.5,
            group_cost: 0.9,
            group_syllable_cost: 0.35,
        }
    }
}

/// Song-specific conversion of the second-based tolerances into beats.
#[derive(Debug, Clone, Copy)]
struct Scale {
    time_beats: f64,
}

impl Scale {
    fn new(header: &SongHeader, options: AlignmentOptions) -> Self {
        let beats_per_second = (header.bpm.max(0.0) * 4.0 / 60.0).max(0.001);
        Self {
            time_beats: (options.time_tolerance_s.max(0.05) * beats_per_second).max(0.5),
        }
    }
}

/// Options plus the song-specific beat conversion, threaded through the DP.
#[derive(Debug, Clone, Copy)]
struct AlignContext {
    options: AlignmentOptions,
    scale: Scale,
}

impl AlignContext {
    fn new(header: &SongHeader, options: AlignmentOptions) -> Self {
        Self {
            options,
            scale: Scale::new(header, options),
        }
    }
}

/// One exported note: lyrics, `~` continuation, or a grouped syllable pair.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AlignedNote {
    pub start_beat: u32,
    pub length_beats: u32,
    pub pitch: i16,
    /// Syllable text, without the word-separator space.
    pub text: String,
    pub starts_word: bool,
    pub is_continuation: bool,
    /// This note carries several syllables compacted by `synthesise_line`.
    pub grouped: bool,
    /// No detected note covered this line; the aligner invented the note.
    pub synthesised: bool,
    pub confidence: f32,
    pub needs_review: bool,
    /// Index of the transcript line this note belongs to.
    pub line: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AlignedPhrase {
    pub notes: Vec<AlignedNote>,
    pub confidence: f32,
    pub needs_review: bool,
    pub break_after: Option<PhraseBreak>,
    pub line: usize,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct AlignmentStats {
    pub syllables: usize,
    pub assigned_syllables: usize,
    /// Detected notes that had to be split between several syllables.
    pub grouped_notes: usize,
    pub continuations: usize,
    /// Notes that could not become a continuation because nothing was sung yet.
    pub dropped_notes: usize,
    pub synthesised_notes: usize,
    pub review_notes: usize,
}

impl AlignmentStats {
    /// True when every syllable was placed and nothing needs review.
    pub fn is_clean(&self) -> bool {
        self.assigned_syllables == self.syllables
            && self.review_notes == 0
            && self.dropped_notes == 0
            && self.synthesised_notes == 0
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Alignment {
    pub phrases: Vec<AlignedPhrase>,
    pub stats: AlignmentStats,
}

impl Alignment {
    pub fn notes(&self) -> impl Iterator<Item = &AlignedNote> {
        self.phrases.iter().flat_map(|phrase| phrase.notes.iter())
    }

    pub fn note_count(&self) -> usize {
        self.phrases.iter().map(|phrase| phrase.notes.len()).sum()
    }

    /// The editor shows "ready to export" only when nothing needs review.
    pub fn is_ready(&self) -> bool {
        !self.phrases.is_empty() && self.stats.is_clean()
    }

    /// `(phrase, note)` pairs the editor must highlight.
    pub fn review_notes(&self) -> Vec<(usize, usize)> {
        let mut result = Vec::new();
        for (phrase_index, phrase) in self.phrases.iter().enumerate() {
            for (note_index, note) in phrase.notes.iter().enumerate() {
                if note.needs_review {
                    result.push((phrase_index, note_index));
                }
            }
        }
        result
    }

    pub fn to_song_doc(&self, header: SongHeader) -> SongDoc {
        SongDoc {
            header,
            phrases: self
                .phrases
                .iter()
                .map(|phrase| Phrase {
                    notes: phrase
                        .notes
                        .iter()
                        .map(|note| Note {
                            kind: NoteKind::Normal,
                            start_beat: note.start_beat,
                            length_beats: note.length_beats,
                            pitch: note.pitch,
                            text: note.text.clone(),
                            starts_word: note.starts_word,
                            confidence: Some(note.confidence),
                        })
                        .collect(),
                    break_after: phrase.break_after.clone(),
                })
                .collect(),
            has_end_marker: true,
            is_duet: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
struct Anchor {
    text: String,
    start_beat: f64,
    end_beat: f64,
    confidence: f32,
    known: bool,
    starts_word: bool,
    line: usize,
}

/// Aligns the transcript with the quantized notes.
pub fn align(
    transcript: &Transcript,
    song: &QuantizedSong,
    header: &SongHeader,
    syllabifier: &dyn Syllabifier,
    options: AlignmentOptions,
) -> Alignment {
    align_with_voiced(transcript, song, header, syllabifier, options, None)
}

/// Aligns the transcript, using detected voiced segments to correct lyrics that
/// were timed against a different upload of the same song.
pub fn align_with_voiced(
    transcript: &Transcript,
    song: &QuantizedSong,
    header: &SongHeader,
    syllabifier: &dyn Syllabifier,
    options: AlignmentOptions,
    voiced: Option<&[(f64, f64)]>,
) -> Alignment {
    let offset_s = match voiced {
        Some(segments) if !transcript.word_timed => estimate_offset_s(transcript, segments),
        _ => 0.0,
    };
    let offset_beats = offset_s * header.bpm.max(0.0) * 4.0 / 60.0;

    let anchors: Vec<Vec<Anchor>> = transcript
        .lines
        .iter()
        .enumerate()
        .map(|(line_index, line)| {
            let mut anchors =
                anchors_for_line(line, &transcript.language, header, syllabifier, line_index);
            if offset_beats != 0.0 {
                for anchor in &mut anchors {
                    anchor.start_beat = (anchor.start_beat + offset_beats).max(0.0);
                    anchor.end_beat = (anchor.end_beat + offset_beats).max(anchor.start_beat);
                }
            }
            anchors
        })
        .collect();

    let stats = AlignmentStats {
        syllables: anchors.iter().map(Vec::len).sum(),
        ..AlignmentStats::default()
    };

    let context = AlignContext::new(header, options);
    let mut stats = stats;
    let mut notes: Vec<AlignedNote> = Vec::new();
    let mut cursor = 0_usize;
    // Each line owns the notes that start between its own first syllable and
    // the first syllable of the next line. Line starts are the most reliable
    // part of line-synced lyrics; per-word times inside a line are guesses, so
    // they must not decide which notes belong to which line.
    let line_starts: Vec<f64> = anchors
        .iter()
        .map(|line| {
            line.first()
                .map(|anchor| anchor.start_beat)
                .unwrap_or(f64::NAN)
        })
        .collect();

    for (line_index, line_anchors) in anchors.iter().enumerate() {
        if line_anchors.is_empty() {
            continue;
        }
        // Always give the line one candidate, even if it starts late.
        let line_start = line_anchors
            .first()
            .map(|anchor| anchor.start_beat)
            .unwrap_or(0.0);
        let next_start = line_starts[line_index + 1..]
            .iter()
            .copied()
            .find(|value| value.is_finite());

        let begin = cursor;
        let mut end = cursor;
        while end < song.notes.len() {
            if let Some(next_start) = next_start {
                if (song.notes[end].start_beat as f64) >= next_start {
                    break;
                }
            }
            end += 1;
        }
        if end == begin && begin < song.notes.len() {
            end = begin + 1;
        }
        cursor = end;

        let window: Vec<QuantizedNote> = song.notes[begin..end].to_vec();
        if window.is_empty() {
            synthesise_line(line_anchors, line_start, &mut notes, &mut stats, context);
        } else {
            let cap_beat = voiced
                .and_then(|segments| {
                    let start_s = seconds(header, line_start);
                    segments
                        .iter()
                        .find(|(segment_start, segment_end)| {
                            start_s >= *segment_start - 0.3 && start_s <= *segment_end
                        })
                        .map(|(_, segment_end)| beats(header, *segment_end))
                })
                .unwrap_or(f64::INFINITY);
            assign_line(
                line_anchors,
                &window,
                &mut notes,
                &mut stats,
                context,
                cap_beat,
            );
        }
    }

    enforce_monotonic(&mut notes);
    let phrases = build_phrases(notes, header, options);

    Alignment { phrases, stats }
}

fn anchors_for_line(
    line: &TranscriptLine,
    language: &str,
    header: &SongHeader,
    syllabifier: &dyn Syllabifier,
    line_index: usize,
) -> Vec<Anchor> {
    let mut anchors = Vec::new();
    for word in &line.words {
        if word.is_empty() {
            continue;
        }
        let split = if word.syllables.is_empty() {
            syllabifier.split(language, &word.text)
        } else {
            crate::transcript_edit::SyllableSplit {
                syllables: word.syllables.clone(),
                known: true,
            }
        };
        let syllables: Vec<&String> = split
            .syllables
            .iter()
            .filter(|syllable| !syllable.trim().is_empty())
            .collect();
        if syllables.is_empty() {
            continue;
        }

        let weights: Vec<f64> = syllables
            .iter()
            .map(|syllable| syllable.chars().count().max(1) as f64)
            .collect();
        let total: f64 = weights.iter().sum::<f64>().max(1.0);
        let span = (word.end_s - word.start_s).max(0.0);
        let mut consumed = 0.0;

        for (index, syllable) in syllables.iter().enumerate() {
            let weight = weights[index];
            let start_s = word.start_s + span * (consumed / total);
            let end_s = word.start_s + span * ((consumed + weight) / total);
            consumed += weight;

            let confidence = if split.known {
                word.confidence
            } else {
                word.confidence * 0.65
            };
            anchors.push(Anchor {
                text: (*syllable).clone(),
                start_beat: beats(header, start_s),
                end_beat: beats(header, end_s).max(beats(header, start_s) + 0.25),
                confidence: confidence.clamp(0.0, 1.0),
                known: split.known,
                starts_word: index == 0,
                line: line_index,
            });
        }
    }
    anchors
}

fn beats(header: &SongHeader, seconds: f64) -> f64 {
    ms_to_beat(header, seconds * 1000.0).max(0.0)
}

fn seconds(header: &SongHeader, beat: f64) -> f64 {
    if header.bpm <= 0.0 {
        return 0.0;
    }
    (header.gap_ms as f64 + beat * 60_000.0 / (header.bpm * 4.0)) / 1000.0
}

/// Global time shift, in seconds, between line-synced lyrics and the audio.
///
/// Free online lyric databases often carry timings from a different upload:
/// same song, extra intro, different interlude. Line starts are matched to the
/// nearest vocal onset (a few milliseconds early is normal, so onsets well
/// before the line are ignored) and the median difference is kept, which
/// discards the odd block that happens to start at the right time.
fn estimate_offset_s(transcript: &Transcript, voiced: &[(f64, f64)]) -> f64 {
    if voiced.len() < 2 {
        return 0.0;
    }
    let mut diffs: Vec<f64> = Vec::new();
    for start in block_starts(transcript) {
        let nearest = voiced
            .iter()
            .map(|(segment_start, _)| *segment_start)
            .filter(|segment_start| {
                let difference = segment_start - start;
                (-1.0..=8.0).contains(&difference)
            })
            .min_by(|left, right| (left - start).abs().total_cmp(&(right - start).abs()));
        if let Some(nearest) = nearest {
            diffs.push(nearest - start);
        }
    }
    if diffs.len() < 2 {
        return 0.0;
    }
    diffs.sort_by(f64::total_cmp);
    let median = diffs[diffs.len() / 2];
    let inliers: Vec<f64> = diffs
        .iter()
        .copied()
        .filter(|difference| (difference - median).abs() <= 1.0)
        .collect();
    if inliers.is_empty() {
        return 0.0;
    }
    let mean = inliers.iter().sum::<f64>() / inliers.len() as f64;
    if (0.3..=8.0).contains(&mean.abs()) {
        mean
    } else {
        0.0
    }
}

/// First line of every stanza. Consecutive lines share their boundary, so a
/// real silence between a line end and the next start marks a new stanza.
fn block_starts(transcript: &Transcript) -> Vec<f64> {
    let mut starts = Vec::new();
    let mut previous_end: Option<f64> = None;
    for line in &transcript.lines {
        if line.words.is_empty() {
            continue;
        }
        let new_block = previous_end
            .map(|end| line.start_s - end > 0.35)
            .unwrap_or(true);
        if new_block {
            starts.push(line.start_s);
        }
        previous_end = Some(line.end_s);
    }
    starts
}

#[derive(Debug, Clone, Copy)]
enum LineStep {
    /// `count` syllables starting at `start` share the note at `note`.
    Syllables {
        start: usize,
        count: usize,
        note: usize,
    },
    /// The note carries no new lyrics and becomes a continuation.
    NoteOnly { note: usize },
}

fn assign_line(
    anchors: &[Anchor],
    notes: &[QuantizedNote],
    output: &mut Vec<AlignedNote>,
    stats: &mut AlignmentStats,
    context: AlignContext,
    cap_beat: f64,
) {
    if anchors.is_empty() || notes.is_empty() {
        return;
    }
    let steps = plan(anchors, notes, context);
    if steps.is_empty() {
        // The line does not fit in its window (usually because the provided
        // timing is too early and only a couple of notes overlap it). Losing
        // the lyrics is worse than a rough fit, so force every syllable onto
        // the available notes and flag the result for review.
        force_assign_line(anchors, notes, output, stats, context, cap_beat);
        return;
    }

    let step_starts: Vec<u32> = steps
        .iter()
        .map(|step| match step {
            LineStep::Syllables { note, .. } | LineStep::NoteOnly { note } => {
                notes[*note].start_beat
            }
        })
        .collect();

    for (index, step) in steps.iter().enumerate() {
        let span = Span {
            limit_beat: step_starts.get(index + 1).copied().unwrap_or(u32::MAX),
            cap_beat,
        };
        match *step {
            LineStep::Syllables { start, count, note } => push_syllable_note(
                SyllableGroup {
                    anchors,
                    start,
                    count,
                },
                &notes[note],
                output,
                stats,
                context,
                span,
            ),
            LineStep::NoteOnly { note } => push_continuation(&notes[note], output, stats),
        }
    }
}

/// Emits a `~` continuation note, or counts a drop when nothing was sung yet.
fn push_continuation(
    note_data: &QuantizedNote,
    output: &mut Vec<AlignedNote>,
    stats: &mut AlignmentStats,
) {
    let Some(previous) = output.last() else {
        stats.dropped_notes += 1;
        return;
    };
    let confidence = (previous.confidence * 0.9).clamp(0.0, 1.0);
    let line = previous.line;
    output.push(AlignedNote {
        start_beat: note_data.start_beat,
        length_beats: note_data.length_beats.max(1),
        pitch: note_data.pitch,
        text: CONTINUATION_TEXT.to_string(),
        starts_word: false,
        is_continuation: true,
        grouped: false,
        synthesised: false,
        confidence,
        needs_review: false,
        line,
    });
    stats.continuations += 1;
}

/// Emits the `count` consecutive syllables that fall on one detected note.
///
/// One detected note that has to carry several syllables.
#[derive(Clone, Copy)]
struct SyllableGroup<'a> {
    anchors: &'a [Anchor],
    start: usize,
    count: usize,
}

/// How far a split note may grow: until the next note of the line and, at the
/// very end of the line, until the vocal segment stops.
#[derive(Clone, Copy)]
struct Span {
    limit_beat: u32,
    cap_beat: f64,
}

/// A detected note covers a stretch of singing, not a single syllable: the
/// pitch tracker merges all the syllables sung on the same pitch. UltraStar
/// needs one note per syllable, so the note is split into `count` slices of at
/// least one beat, keeping the detected pitch. Syllables with no room left are
/// flagged for review.
fn push_syllable_note(
    group: SyllableGroup<'_>,
    note_data: &QuantizedNote,
    output: &mut Vec<AlignedNote>,
    stats: &mut AlignmentStats,
    context: AlignContext,
    span: Span,
) {
    let count = group.count;
    let syllables = &group.anchors[group.start..group.start + count];
    let detected_end = note_data.start_beat + note_data.length_beats.max(1);
    // Line-synced lyrics know roughly where the group ends; using that avoids
    // cramming a whole word into a single short detected note. Never grow past
    // the next note or the end of the sung segment.
    let desired_end = syllables
        .last()
        .map(|anchor| anchor.end_beat.ceil())
        .unwrap_or(detected_end as f64);
    let limit = (span.limit_beat as f64)
        .min(span.cap_beat)
        .min(desired_end)
        .max(detected_end as f64);
    let total = ((limit - note_data.start_beat as f64).floor() as u32)
        .max(note_data.length_beats.max(count as u32));
    let mut consumed = 0_u32;
    let mut flagged = 0_usize;

    for (index, anchor) in syllables.iter().enumerate() {
        let index = index as u32;
        let end = if index + 1 == count as u32 {
            total
        } else {
            ((index + 1) * total) / count as u32
        };
        let length = end.saturating_sub(consumed).max(1);
        let start_beat = note_data.start_beat + consumed.min(total - 1);
        consumed = end;

        let confidence =
            syllable_confidence(anchor, start_beat as f64, false, context.scale).clamp(0.0, 1.0);
        let needs_review = confidence < context.options.min_confidence || !anchor.known;
        if needs_review {
            flagged += 1;
        }
        output.push(AlignedNote {
            start_beat,
            length_beats: length,
            pitch: note_data.pitch,
            text: anchor.text.clone(),
            starts_word: anchor.starts_word,
            is_continuation: false,
            grouped: false,
            synthesised: false,
            confidence,
            needs_review,
            line: anchor.line,
        });
    }

    stats.assigned_syllables += count;
    stats.review_notes += flagged;
    if count > 1 {
        stats.grouped_notes += 1;
    }
}

/// Places every syllable on the available notes even when the DP cannot find a
/// legal path (too many syllables for the window). The split is proportional:
/// each note takes an equal share, and leftover notes become continuations.
fn force_assign_line(
    anchors: &[Anchor],
    notes: &[QuantizedNote],
    output: &mut Vec<AlignedNote>,
    stats: &mut AlignmentStats,
    context: AlignContext,
    cap_beat: f64,
) {
    let mut assigned = 0;
    for (index, note_data) in notes.iter().enumerate() {
        let notes_left = notes.len() - index;
        let remaining = anchors.len() - assigned;
        if remaining == 0 {
            push_continuation(note_data, output, stats);
            continue;
        }
        let count = remaining.div_ceil(notes_left).max(1);
        push_syllable_note(
            SyllableGroup {
                anchors,
                start: assigned,
                count,
            },
            note_data,
            output,
            stats,
            context,
            Span {
                limit_beat: notes
                    .get(index + 1)
                    .map(|note| note.start_beat)
                    .unwrap_or(u32::MAX),
                cap_beat,
            },
        );
        assigned += count;
    }
}

fn synthesise_line(
    anchors: &[Anchor],
    line_start: f64,
    output: &mut Vec<AlignedNote>,
    stats: &mut AlignmentStats,
    context: AlignContext,
) {
    let start = output
        .last()
        .map(|note| note.start_beat + note.length_beats)
        .unwrap_or_else(|| line_start.round().max(0.0) as u32);
    let mut text = String::new();
    let mut confidence_sum = 0.0_f32;
    for (index, anchor) in anchors.iter().enumerate() {
        if index > 0 && anchor.starts_word {
            text.push(' ');
        }
        text.push_str(&anchor.text);
        confidence_sum += syllable_confidence(anchor, start as f64, true, context.scale);
    }
    let confidence = (confidence_sum / anchors.len() as f32 * 0.5).clamp(0.0, 1.0);
    output.push(AlignedNote {
        start_beat: start,
        length_beats: 1,
        pitch: 0,
        text,
        starts_word: anchors[0].starts_word,
        is_continuation: false,
        grouped: true,
        synthesised: true,
        confidence,
        needs_review: true,
        line: anchors[0].line,
    });
    stats.assigned_syllables += anchors.len();
    stats.grouped_notes += 1;
    stats.synthesised_notes += 1;
    stats.review_notes += 1;
}

/// Confidence of one syllable on a note: word confidence, pitch/time fit and a
/// penalty for grouped syllables and unknown words.
fn syllable_confidence(anchor: &Anchor, note_start: f64, grouped: bool, scale: Scale) -> f32 {
    let distance = (note_start - anchor.start_beat).abs();
    let tolerance = scale.time_beats.max(0.25);
    let time_score = (-(distance / tolerance).powi(2)).exp() as f32;
    let mut confidence = anchor.confidence * (0.35 + 0.65 * time_score);
    if !anchor.known {
        confidence *= 0.75;
    }
    if grouped {
        confidence *= 0.7;
    }
    confidence.clamp(0.0, 1.0)
}

fn plan(anchors: &[Anchor], notes: &[QuantizedNote], context: AlignContext) -> Vec<LineStep> {
    let options = context.options;
    let syllable_count = anchors.len();
    let note_count = notes.len();
    let width = note_count + 1;
    let mut cost = vec![f64::INFINITY; (syllable_count + 1) * width];
    let mut from: Vec<Option<LineStep>> = vec![None; (syllable_count + 1) * width];

    cost[0] = 0.0;
    for note_index in 0..note_count {
        for syllable_index in 0..=syllable_count {
            let current = cost[syllable_index * width + note_index];
            if !current.is_finite() {
                continue;
            }

            if syllable_index < syllable_count {
                let next = current
                    + match_cost(
                        &anchors[syllable_index],
                        &notes[note_index],
                        note_index,
                        notes,
                        context.scale,
                    );
                relax(
                    &mut cost,
                    &mut from,
                    (syllable_index + 1) * width + note_index + 1,
                    next,
                    LineStep::Syllables {
                        start: syllable_index,
                        count: 1,
                        note: note_index,
                    },
                );
            }

            relax(
                &mut cost,
                &mut from,
                syllable_index * width + note_index + 1,
                current + options.skip_note_cost,
                LineStep::NoteOnly { note: note_index },
            );

            let max_group = options
                .max_grouped
                .max(2)
                .min(syllable_count - syllable_index);
            for count in 2..=max_group {
                let group = &anchors[syllable_index..syllable_index + count];
                let next = current
                    + options.group_cost
                    + options.group_syllable_cost * (count - 1) as f64
                    + match_cost(
                        &group[0],
                        &notes[note_index],
                        note_index,
                        notes,
                        context.scale,
                    );
                relax(
                    &mut cost,
                    &mut from,
                    (syllable_index + count) * width + note_index + 1,
                    next,
                    LineStep::Syllables {
                        start: syllable_index,
                        count,
                        note: note_index,
                    },
                );
            }
        }
    }

    let end_index = syllable_count * width + note_count;
    if !cost[end_index].is_finite() {
        return Vec::new();
    }

    let mut steps = Vec::new();
    let (mut syllable_index, mut note_index) = (syllable_count, note_count);
    while syllable_index > 0 || note_index > 0 {
        let Some(step) = from[syllable_index * width + note_index] else {
            break;
        };
        match step {
            LineStep::Syllables { start, note, .. } => {
                syllable_index = start;
                note_index = note;
            }
            LineStep::NoteOnly { note } => {
                note_index = note;
            }
        }
        steps.push(step);
    }
    steps.reverse();
    steps
}

fn relax(
    cost: &mut [f64],
    from: &mut [Option<LineStep>],
    index: usize,
    candidate: f64,
    step: LineStep,
) {
    if candidate + 1e-9 < cost[index] {
        cost[index] = candidate;
        from[index] = Some(step);
    }
}

fn match_cost(
    anchor: &Anchor,
    note: &QuantizedNote,
    note_index: usize,
    notes: &[QuantizedNote],
    scale: Scale,
) -> f64 {
    let tolerance = scale.time_beats.max(0.25);
    let distance = (note.start_beat as f64 - anchor.start_beat).abs();
    let time_cost = (distance / tolerance).min(4.0);

    let expected = (anchor.end_beat - anchor.start_beat).clamp(0.5, 8.0);
    let duration_cost =
        ((note.length_beats as f64 - expected).abs() / (expected + 1.0)).min(2.0) * 0.4;

    let pitch_cost = note_index
        .checked_sub(1)
        .map(|previous| ((notes[previous].pitch - note.pitch).abs() as f64 / 12.0).min(1.0) * 0.1)
        .unwrap_or(0.0);

    time_cost + duration_cost + pitch_cost
}

/// Guarantees increasing starts and a minimum length of one beat.
fn enforce_monotonic(notes: &mut [AlignedNote]) {
    let mut next_start = 0_u32;
    for note in notes.iter_mut() {
        if note.start_beat < next_start {
            note.start_beat = next_start;
        }
        note.length_beats = note.length_beats.max(1);
        next_start = note.start_beat.saturating_add(note.length_beats);
    }
}

fn build_phrases(
    notes: Vec<AlignedNote>,
    header: &SongHeader,
    options: AlignmentOptions,
) -> Vec<AlignedPhrase> {
    if notes.is_empty() {
        return Vec::new();
    }
    let gap_beats = beats(header, options.phrase_gap_s.max(0.0));

    let mut phrases: Vec<AlignedPhrase> = Vec::new();
    let mut current: Vec<AlignedNote> = Vec::new();
    let mut current_line = notes[0].line;

    for note in notes {
        let line_changed = note.line != current_line;
        let long_pause = current
            .last()
            .map(|previous| {
                let previous_end = (previous.start_beat + previous.length_beats) as f64;
                note.start_beat as f64 - previous_end > gap_beats
            })
            .unwrap_or(false);

        if !current.is_empty() && (line_changed || long_pause) {
            phrases.push(finish_phrase(std::mem::take(&mut current)));
        }
        current_line = note.line;
        current.push(note);
    }
    if !current.is_empty() {
        phrases.push(finish_phrase(current));
    }

    for index in 0..phrases.len().saturating_sub(1) {
        let end_beat = phrases[index]
            .notes
            .last()
            .map(|note| note.start_beat + note.length_beats)
            .unwrap_or(0);
        let next_start = phrases[index + 1]
            .notes
            .first()
            .map(|note| note.start_beat)
            .unwrap_or(end_beat);
        phrases[index].break_after = Some(PhraseBreak {
            start_beat: end_beat,
            end_beat: (next_start > end_beat).then_some(next_start),
        });
    }

    phrases
}

fn finish_phrase(notes: Vec<AlignedNote>) -> AlignedPhrase {
    let line = notes.first().map(|note| note.line).unwrap_or(0);
    let confident: Vec<f32> = notes
        .iter()
        .filter(|note| !note.is_continuation)
        .map(|note| note.confidence)
        .collect();
    let confidence = if confident.is_empty() {
        0.0
    } else {
        confident.iter().sum::<f32>() / confident.len() as f32
    };
    let needs_review = notes.iter().any(|note| note.needs_review);
    AlignedPhrase {
        notes,
        confidence,
        needs_review,
        break_after: None,
        line,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use usdx_domain::{TranscriptLine, TranscriptWord};

    fn header() -> SongHeader {
        // 120 BPM in the file means one beat every 125 ms.
        SongHeader {
            title: "Canción".into(),
            artist: "Artista".into(),
            audio: "audio.mp3".into(),
            bpm: 120.0,
            gap_ms: 0,
            ..SongHeader::default()
        }
    }

    fn word(
        text: &str,
        start_s: f64,
        end_s: f64,
        confidence: f32,
        syllables: &[&str],
    ) -> TranscriptWord {
        TranscriptWord {
            text: text.into(),
            start_s,
            end_s,
            confidence,
            syllables: syllables.iter().map(|value| value.to_string()).collect(),
            ..TranscriptWord::default()
        }
    }

    fn make_transcript(lines: Vec<Vec<TranscriptWord>>) -> Transcript {
        Transcript {
            language: "es".into(),
            language_confidence: 0.95,
            lines: lines
                .into_iter()
                .map(|words| {
                    let mut line = TranscriptLine::new(words);
                    line.refresh_bounds();
                    line
                })
                .collect(),
            discarded_segments: 0,
            word_timed: true,
        }
    }

    fn note(start_beat: u32, length_beats: u32, pitch: i16, confidence: f32) -> QuantizedNote {
        QuantizedNote {
            start_beat,
            length_beats,
            pitch,
            confidence,
            adjusted: false,
        }
    }

    fn quantized(notes: Vec<QuantizedNote>) -> QuantizedSong {
        QuantizedSong {
            notes,
            adjustments: Vec::new(),
        }
    }

    fn run(transcript: &Transcript, notes: Vec<QuantizedNote>) -> Alignment {
        align(
            transcript,
            &quantized(notes),
            &header(),
            crate::syllabify::dictionary_syllabifier(),
            AlignmentOptions::default(),
        )
    }

    #[test]
    fn one_syllable_per_note_keeps_word_order() {
        let transcript = make_transcript(vec![vec![word("hola", 0.0, 0.5, 0.9, &["ho", "la"])]]);
        let alignment = run(&transcript, vec![note(0, 2, 0, 0.9), note(2, 2, 2, 0.9)]);

        let notes: Vec<_> = alignment.notes().collect();
        assert_eq!(notes.len(), 2);
        assert_eq!(notes[0].text, "ho");
        assert_eq!(notes[1].text, "la");
        assert!(notes[0].starts_word);
        assert!(!notes[1].starts_word);
        assert!(notes.iter().all(|note| !note.is_continuation));
        assert!(notes.iter().all(|note| note.start_beat <= 4));
        assert_eq!(alignment.stats.assigned_syllables, 2);
        assert_eq!(alignment.stats.syllables, 2);
        assert!(alignment.is_ready(), "{:?}", alignment.stats);
    }

    #[test]
    fn line_with_too_few_notes_is_forced_instead_of_dropped() {
        // Eleven syllables but a single note inside the window: the DP cannot
        // place them all (max_grouped is 4), so the line must still survive.
        // The note is stretched over the line and split into syllables.
        let transcript = make_transcript(vec![vec![
            word("soy", 0.0, 0.2, 0.9, &["soy"]),
            word("un", 0.2, 0.3, 0.9, &["un"]),
            word("amante", 0.3, 0.7, 0.9, &["a", "man", "te"]),
            word("del", 0.7, 0.9, 0.9, &["del"]),
            word("caos", 0.9, 1.2, 0.9, &["ca", "os"]),
            word("voz", 1.2, 1.4, 0.9, &["voz"]),
            word("final", 1.4, 1.8, 0.9, &["fi", "nal"]),
        ]]);
        let alignment = run(&transcript, vec![note(0, 2, 0, 0.9)]);

        assert!(!alignment.phrases.is_empty(), "line was dropped");
        assert_eq!(alignment.stats.syllables, 11);
        assert_eq!(
            alignment.stats.assigned_syllables, alignment.stats.syllables,
            "{:?}",
            alignment.stats
        );
        assert_eq!(alignment.stats.synthesised_notes, 0);
        let syllables: Vec<_> = alignment
            .notes()
            .filter(|note| !note.is_continuation)
            .collect();
        assert_eq!(syllables.len(), 11);
        assert!(syllables.iter().all(|note| note.length_beats >= 1));
        // The single note covers the whole line instead of one beat.
        let last = syllables.last().expect("a syllable");
        assert!(
            last.start_beat + last.length_beats > 10,
            "the forced line was not stretched: {last:?}"
        );

        let written: String = syllables
            .iter()
            .map(|note| note.text.clone())
            .collect::<Vec<_>>()
            .join(" ");
        let written: String = written.chars().filter(|c| !c.is_whitespace()).collect();
        assert_eq!(written, "soyunamantedelcaosvozfinal");
    }

    #[test]
    fn extra_notes_become_continuations() {
        let transcript = make_transcript(vec![vec![word("sola", 0.0, 0.5, 0.9, &["so", "la"])]]);
        let alignment = run(
            &transcript,
            vec![note(0, 2, 0, 0.9), note(2, 2, 2, 0.9), note(4, 2, 4, 0.9)],
        );

        let notes: Vec<_> = alignment.notes().collect();
        assert_eq!(notes.len(), 3);
        assert!(notes[2].is_continuation);
        assert_eq!(notes[2].text, CONTINUATION_TEXT);
        assert_eq!(alignment.stats.continuations, 1);
        assert_eq!(alignment.stats.assigned_syllables, 2);
    }

    #[test]
    fn extra_syllables_split_the_detected_note() {
        let transcript = make_transcript(vec![vec![word(
            "canción",
            0.0,
            0.6,
            0.9,
            &["can", "ci", "ón"],
        )]]);
        let alignment = run(&transcript, vec![note(0, 3, 0, 0.9), note(4, 2, 2, 0.9)]);

        assert_eq!(alignment.stats.assigned_syllables, 3);
        assert_eq!(alignment.stats.grouped_notes, 1);

        let notes: Vec<_> = alignment.notes().collect();
        assert_eq!(notes.len(), 3);
        let texts: Vec<&str> = notes.iter().map(|note| note.text.as_str()).collect();
        assert_eq!(texts, vec!["can", "ci", "ón"]);
        // The two syllables on the first detected note keep its pitch and grow
        // towards the next note, which starts at beat 4.
        assert_eq!(notes[0].start_beat, 0);
        assert_eq!(
            notes[1].start_beat,
            notes[0].start_beat + notes[0].length_beats
        );
        assert_eq!(notes[0].pitch, notes[1].pitch);
        assert_eq!(notes[0].length_beats + notes[1].length_beats, 4);
        assert!(notes.iter().all(|note| note.length_beats >= 1));
        assert!(alignment.is_ready(), "{:?}", alignment.stats);
    }

    #[test]
    fn line_changes_and_long_pauses_start_new_phrases() {
        let transcript = make_transcript(vec![
            vec![word("uno", 0.0, 0.4, 0.9, &["u", "no"])],
            vec![word("dos", 1.6, 2.0, 0.9, &["dos"])],
        ]);
        let alignment = run(
            &transcript,
            vec![note(0, 2, 0, 0.9), note(2, 2, 0, 0.9), note(16, 3, 0, 0.9)],
        );

        assert_eq!(alignment.phrases.len(), 2);
        let first_break = alignment.phrases[0].break_after.clone().expect("break");
        assert_eq!(first_break.start_beat, 4);
        assert_eq!(first_break.end_beat, Some(16));

        // A long pause inside one line also breaks the phrase.
        let transcript = make_transcript(vec![vec![
            word("uno", 0.0, 0.4, 0.9, &["u", "no"]),
            word("dos", 1.6, 2.0, 0.9, &["dos"]),
        ]]);
        let alignment = run(
            &transcript,
            vec![note(0, 2, 0, 0.9), note(2, 2, 0, 0.9), note(16, 3, 0, 0.9)],
        );
        assert_eq!(alignment.phrases.len(), 2);
    }

    #[test]
    fn low_confidence_words_are_flagged_for_review() {
        let strong = make_transcript(vec![vec![word("bien", 0.0, 0.4, 0.95, &["bien"])]]);
        let high = run(&strong, vec![note(0, 3, 0, 0.95)]);
        assert!(high.is_ready(), "{:?}", high.stats);
        assert!(high.review_notes().is_empty());
        assert!(high.phrases[0].confidence > 0.55);

        let weak = make_transcript(vec![vec![word("bien", 0.0, 0.4, 0.2, &["bien"])]]);
        let low = run(&weak, vec![note(0, 3, 0, 0.2)]);
        assert!(!low.is_ready());
        assert_eq!(low.review_notes().len(), 1);
        assert!(low.phrases[0].needs_review);
        assert!(low.phrases[0].confidence < 0.55);
    }

    #[test]
    fn aligned_notes_can_be_written_and_parsed_back() {
        let transcript = make_transcript(vec![vec![word(
            "hola mundo",
            0.0,
            0.8,
            0.9,
            &["ho", "la", "mun", "do"],
        )]]);
        let alignment = run(
            &transcript,
            vec![
                note(0, 2, 0, 0.9),
                note(2, 2, 1, 0.9),
                note(4, 2, 2, 0.9),
                note(6, 2, 3, 0.9),
            ],
        );
        let document = alignment.to_song_doc(header());
        let text = usdx_format::write(&document).expect("write");
        let parsed = usdx_format::parse(&text);
        assert_eq!(parsed.doc.note_count(), 4);
        assert!(parsed.warnings.is_empty(), "{:?}", parsed.warnings);
        let written: Vec<(u32, u32, i16)> = parsed
            .doc
            .notes()
            .map(|note| (note.start_beat, note.length_beats, note.pitch))
            .collect();
        assert_eq!(written, vec![(0, 2, 0), (2, 2, 1), (4, 2, 2), (6, 2, 3)]);
    }

    #[test]
    fn generated_songs_keep_every_syllable_ordered_and_disjoint() {
        let mut state = 0x2545_f491_u32;
        let mut next = |max: u32| {
            state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            (state >> 8) % max
        };

        for _ in 0..20 {
            let line_count = 1 + next(4) as usize;
            let mut lines = Vec::new();
            let mut notes = Vec::new();
            let mut beat = 0_u32;
            let mut seconds = 0.0_f64;

            for _ in 0..line_count {
                let word_count = 1 + next(3) as usize;
                let mut words = Vec::new();
                for _ in 0..word_count {
                    let syllables = ["ka", "ra", "to", "mi"];
                    let count = 1 + next(4) as usize;
                    let chosen: Vec<&str> = syllables[..count].to_vec();
                    let text: String = chosen.concat();
                    let start_s = seconds;
                    let end_s = seconds + 0.15 * count as f64;
                    seconds = end_s;
                    words.push(word(&text, start_s, end_s, 0.9, &chosen));

                    for _ in 0..count {
                        notes.push(note(beat, 1 + next(2), (next(12) as i16) - 6, 0.9));
                        beat += 2;
                    }
                }
                lines.push(words);
                beat += 2;
            }

            let source: String = lines
                .iter()
                .flatten()
                .map(|word| word.text.clone())
                .collect::<Vec<_>>()
                .join("");
            let transcript = make_transcript(lines);
            let alignment = run(&transcript, notes);

            assert_eq!(
                alignment.stats.assigned_syllables, alignment.stats.syllables,
                "lost syllables: {:?}",
                alignment.stats
            );

            let mut previous_start = 0;
            for note in alignment.notes() {
                assert!(note.start_beat >= previous_start, "non-monotonic starts");
                assert!(note.length_beats >= 1);
                previous_start = note.start_beat;
            }

            for phrase in &alignment.phrases {
                for pair in phrase.notes.windows(2) {
                    let end = pair[0].start_beat + pair[0].length_beats;
                    assert!(
                        pair[1].start_beat >= end,
                        "overlap inside a phrase: {pair:?}"
                    );
                }
            }

            let written: String = alignment
                .notes()
                .filter(|note| !note.is_continuation)
                .map(|note| note.text.clone())
                .collect::<Vec<_>>()
                .join("");
            let written: String = written.chars().filter(|c| !c.is_whitespace()).collect();
            let source: String = source.chars().filter(|c| !c.is_whitespace()).collect();
            assert_eq!(written, source, "text changed during alignment");
        }
    }

    #[test]
    fn line_synced_lyrics_are_shifted_onto_the_detected_vocal_onsets() {
        // Three stanzas whose text was timed 2.4 s before this upload.
        let line = |start: f64, end: f64, text: &str| {
            TranscriptLine::new(vec![word(text, start, end, 0.9, &[])])
        };
        let mut transcript = make_transcript(Vec::new());
        transcript.word_timed = false;
        transcript.lines = vec![
            line(0.0, 2.0, "uno"),
            line(2.0, 4.0, "dos"),
            line(5.0, 7.0, "tres"),
            line(8.0, 9.0, "cuatro"),
        ];
        let voiced = vec![(2.4, 6.0), (7.4, 8.8), (10.4, 12.0)];

        let offset = estimate_offset_s(&transcript, &voiced);
        assert!((offset - 2.4).abs() < 0.05, "offset = {offset}");
        assert_eq!(block_starts(&transcript), vec![0.0, 5.0, 8.0]);

        // Lyrics that already match the audio keep their times.
        let voiced = vec![(0.05, 2.0), (5.05, 7.0), (8.05, 9.0)];
        assert_eq!(estimate_offset_s(&transcript, &voiced), 0.0);
    }

    #[test]
    fn word_timed_lyrics_ignore_the_voice_offset() {
        let line = |start: f64, end: f64| {
            TranscriptLine::new(vec![word("hola", start, end, 0.9, &["ho", "la"])])
        };
        let mut transcript = make_transcript(Vec::new());
        transcript.word_timed = true;
        transcript.lines = vec![line(0.0, 0.3), line(1.0, 1.3)];
        let notes = quantized(vec![
            note(19, 2, 0, 0.9),
            note(27, 2, 1, 0.9),
            note(35, 2, 2, 0.9),
            note(43, 2, 3, 0.9),
        ]);
        let voiced = vec![(2.4, 3.0), (3.4, 4.0)];

        let plain = align(
            &transcript,
            &notes,
            &header(),
            crate::syllabify::dictionary_syllabifier(),
            AlignmentOptions::default(),
        );
        let with_voiced = align_with_voiced(
            &transcript,
            &notes,
            &header(),
            crate::syllabify::dictionary_syllabifier(),
            AlignmentOptions::default(),
            Some(&voiced),
        );
        assert_eq!(plain, with_voiced);
    }

    #[test]
    fn documented_example_matches_the_code() {
        let transcript =
            make_transcript(vec![vec![word("canción", 0.0, 0.5, 0.9, &["can", "ción"])]]);
        let alignment = run(&transcript, vec![note(0, 2, 0, 0.9), note(2, 2, 0, 0.9)]);
        let document = include_str!("../../../docs/alignment.md");

        for note in alignment.notes() {
            let line = format!(
                "{} start={} length={} pitch={}",
                note.text, note.start_beat, note.length_beats, note.pitch
            );
            assert!(
                document.contains(&line),
                "documented example is stale, missing `{line}`"
            );
        }
    }
}

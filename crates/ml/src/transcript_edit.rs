//! Review-time edits to a transcript. Timings are preserved; only the edited
//! word loses its syllable split so the aligner recomputes that word alone.

use usdx_domain::Transcript;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum TranscriptEditError {
    #[error("line {0} does not exist")]
    LineOutOfRange(usize),
    #[error("line {line} has no word {word}")]
    WordOutOfRange { line: usize, word: usize },
    #[error("cannot split line {0} at its first word")]
    InvalidSplit(usize),
    #[error("line {0} has no next line to merge with")]
    NoNextLine(usize),
    #[error("word text cannot be empty; delete the word instead")]
    EmptyText,
}

/// Minimal syllabification interface consumed by [`reapply_syllables`].
/// The real implementation lives in [`crate::syllabify`].
pub trait Syllabifier {
    fn syllables(&self, language: &str, word: &str) -> Vec<String>;

    /// Same split plus whether the dictionary recognised the word. Unknown
    /// words use a fallback split and must be flagged for review.
    fn split(&self, language: &str, word: &str) -> SyllableSplit {
        SyllableSplit {
            syllables: self.syllables(language, word),
            known: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyllableSplit {
    pub syllables: Vec<String>,
    /// False when no dictionary covered the word and a fallback rule was used.
    pub known: bool,
}

/// Corrects a word in place, keeping its start and end times.
pub fn correct_word(
    transcript: &mut Transcript,
    line: usize,
    word: usize,
    text: impl Into<String>,
) -> Result<(), TranscriptEditError> {
    let text = text.into();
    if text.trim().is_empty() {
        return Err(TranscriptEditError::EmptyText);
    }
    let target = transcript
        .lines
        .get_mut(line)
        .ok_or(TranscriptEditError::LineOutOfRange(line))?
        .words
        .get_mut(word)
        .ok_or(TranscriptEditError::WordOutOfRange { line, word })?;
    target.set_text(text.trim());
    Ok(())
}

/// Removes one word; the line disappears when no words remain.
pub fn delete_word(
    transcript: &mut Transcript,
    line: usize,
    word: usize,
) -> Result<(), TranscriptEditError> {
    let target = transcript
        .lines
        .get_mut(line)
        .ok_or(TranscriptEditError::LineOutOfRange(line))?;
    if word >= target.words.len() {
        return Err(TranscriptEditError::WordOutOfRange { line, word });
    }
    target.words.remove(word);
    target.refresh_bounds();
    if target.words.is_empty() {
        transcript.lines.remove(line);
    }
    Ok(())
}

/// Splits a line before `word`, producing two lines that share no words.
pub fn split_line(
    transcript: &mut Transcript,
    line: usize,
    word: usize,
) -> Result<(), TranscriptEditError> {
    let target = transcript
        .lines
        .get_mut(line)
        .ok_or(TranscriptEditError::LineOutOfRange(line))?;
    if word == 0 {
        return Err(TranscriptEditError::InvalidSplit(line));
    }
    if word >= target.words.len() {
        return Err(TranscriptEditError::WordOutOfRange { line, word });
    }
    let tail = target.words.split_off(word);
    target.refresh_bounds();
    let mut new_line = usdx_domain::TranscriptLine::new(tail);
    new_line.refresh_bounds();
    transcript.lines.insert(line + 1, new_line);
    Ok(())
}

/// Joins a line with the following one, keeping every word and its timing.
pub fn merge_line_with_next(
    transcript: &mut Transcript,
    line: usize,
) -> Result<(), TranscriptEditError> {
    if line + 1 >= transcript.lines.len() {
        return Err(TranscriptEditError::LineOutOfRange(line));
    }
    let next = transcript.lines.remove(line + 1);
    let target = transcript
        .lines
        .get_mut(line)
        .ok_or(TranscriptEditError::LineOutOfRange(line))?;
    target.words.extend(next.words);
    target.refresh_bounds();
    Ok(())
}

/// Deletes a line; its words no longer take part in alignment or export.
pub fn delete_line(transcript: &mut Transcript, line: usize) -> Result<(), TranscriptEditError> {
    if line >= transcript.lines.len() {
        return Err(TranscriptEditError::LineOutOfRange(line));
    }
    transcript.lines.remove(line);
    Ok(())
}

/// Fills syllables for words that need them. Words whose split is still valid
/// are not touched, so correcting one word re-syllabifies only that word.
pub fn reapply_syllables(transcript: &mut Transcript, syllabifier: &dyn Syllabifier) -> usize {
    let language = transcript.language.clone();
    let mut updated = 0;
    for line in &mut transcript.lines {
        for word in &mut line.words {
            if !word.needs_syllables() {
                continue;
            }
            word.syllables = syllabifier.syllables(&language, &word.text);
            updated += 1;
        }
    }
    updated
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use usdx_domain::{TranscriptLine, TranscriptWord};

    struct CountingSyllabifier {
        calls: RefCell<Vec<String>>,
    }

    impl Syllabifier for CountingSyllabifier {
        fn syllables(&self, _language: &str, word: &str) -> Vec<String> {
            self.calls.borrow_mut().push(word.to_string());
            word.chars()
                .map(|character| character.to_string())
                .collect()
        }
    }

    fn word(text: &str, start: f64, end: f64) -> TranscriptWord {
        TranscriptWord {
            text: text.into(),
            start_s: start,
            end_s: end,
            confidence: 0.9,
            syllables: vec![format!("{text}-split")],
            ..TranscriptWord::default()
        }
    }

    fn transcript() -> Transcript {
        Transcript {
            language: "es".into(),
            language_confidence: 0.9,
            lines: vec![
                TranscriptLine::new(vec![word("hola", 0.0, 0.3), word("mundo", 0.4, 0.8)]),
                TranscriptLine::new(vec![word("adios", 1.0, 1.4)]),
            ],
            discarded_segments: 0,
            word_timed: false,
        }
    }

    #[test]
    fn correcting_a_word_preserves_times_and_only_re_syllabifies_it() {
        let mut transcript = transcript();
        correct_word(&mut transcript, 0, 1, "mondo").unwrap();

        let corrected = &transcript.lines[0].words[1];
        assert_eq!(corrected.text, "mondo");
        assert_eq!(corrected.start_s, 0.4);
        assert_eq!(corrected.end_s, 0.8);
        assert!(corrected.syllables.is_empty());
        assert_eq!(transcript.lines[0].words[0].syllables, vec!["hola-split"]);

        let syllabifier = CountingSyllabifier {
            calls: RefCell::new(Vec::new()),
        };
        let updated = reapply_syllables(&mut transcript, &syllabifier);
        assert_eq!(updated, 1);
        assert_eq!(*syllabifier.calls.borrow(), vec!["mondo"]);
        assert_eq!(transcript.lines[0].words[0].syllables, vec!["hola-split"]);
        assert_eq!(transcript.lines[0].words[1].syllables.len(), 5);
    }

    #[test]
    fn splitting_and_merging_keep_word_times_and_refresh_bounds() {
        let mut transcript = transcript();
        split_line(&mut transcript, 0, 1).unwrap();
        assert_eq!(transcript.lines.len(), 3);
        assert_eq!(transcript.lines[0].text(), "hola");
        assert_eq!(transcript.lines[0].end_s, 0.3);
        assert_eq!(transcript.lines[1].text(), "mundo");
        assert_eq!(transcript.lines[1].start_s, 0.4);
        assert_eq!(transcript.lines[1].words[0].end_s, 0.8);

        merge_line_with_next(&mut transcript, 0).unwrap();
        assert_eq!(transcript.lines.len(), 2);
        assert_eq!(transcript.lines[0].text(), "hola mundo");
        assert_eq!(transcript.lines[0].start_s, 0.0);
        assert_eq!(transcript.lines[0].end_s, 0.8);
    }

    #[test]
    fn deleting_a_line_removes_its_words_from_the_alignment_input() {
        let mut transcript = transcript();
        delete_line(&mut transcript, 0).unwrap();
        assert_eq!(transcript.lines.len(), 1);
        assert_eq!(transcript.text(), "adios");
        assert_eq!(transcript.word_count(), 1);
    }

    #[test]
    fn deleting_the_last_word_removes_the_line() {
        let mut transcript = transcript();
        delete_word(&mut transcript, 1, 0).unwrap();
        assert_eq!(transcript.lines.len(), 1);
        assert_eq!(transcript.lines[0].text(), "hola mundo");
    }

    #[test]
    fn invalid_edits_are_rejected() {
        let mut transcript = transcript();
        assert_eq!(
            correct_word(&mut transcript, 5, 0, "x"),
            Err(TranscriptEditError::LineOutOfRange(5))
        );
        assert_eq!(
            correct_word(&mut transcript, 0, 9, "x"),
            Err(TranscriptEditError::WordOutOfRange { line: 0, word: 9 })
        );
        assert_eq!(
            correct_word(&mut transcript, 0, 0, "  "),
            Err(TranscriptEditError::EmptyText)
        );
        assert_eq!(
            split_line(&mut transcript, 0, 0),
            Err(TranscriptEditError::InvalidSplit(0))
        );
        assert_eq!(
            merge_line_with_next(&mut transcript, 1),
            Err(TranscriptEditError::LineOutOfRange(1))
        );
        assert_eq!(
            delete_line(&mut transcript, 7),
            Err(TranscriptEditError::LineOutOfRange(7))
        );
    }
}

use serde::{Deserialize, Serialize};

/// A recognized word with its time span and confidence.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct TranscriptWord {
    pub text: String,
    /// Start time in seconds.
    pub start_s: f64,
    /// End time in seconds.
    pub end_s: f64,
    /// Mean token probability reported by the engine.
    pub confidence: f32,
    /// True when the word is below the review threshold.
    pub low_confidence: bool,
    /// Syllable split filled by the aligner. Empty means "needs computing".
    pub syllables: Vec<String>,
}

impl TranscriptWord {
    pub fn duration_s(&self) -> f64 {
        (self.end_s - self.start_s).max(0.0)
    }

    pub fn is_empty(&self) -> bool {
        self.text.trim().is_empty()
    }

    /// Marks the word as edited so only this word is re-syllabified.
    pub fn set_text(&mut self, text: impl Into<String>) {
        self.text = text.into();
        self.syllables.clear();
    }

    pub fn needs_syllables(&self) -> bool {
        self.syllables.is_empty() && !self.is_empty()
    }
}

/// A lyric line, normally one Whisper segment.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct TranscriptLine {
    pub start_s: f64,
    pub end_s: f64,
    pub words: Vec<TranscriptWord>,
}

impl TranscriptLine {
    pub fn new(words: Vec<TranscriptWord>) -> Self {
        let start_s = words.first().map(|word| word.start_s).unwrap_or(0.0);
        let end_s = words.last().map(|word| word.end_s).unwrap_or(start_s);
        Self {
            start_s,
            end_s,
            words,
        }
    }

    /// Recomputes the line bounds from its words, preserving word timings.
    pub fn refresh_bounds(&mut self) {
        self.start_s = self.words.first().map(|word| word.start_s).unwrap_or(0.0);
        self.end_s = self
            .words
            .last()
            .map(|word| word.end_s)
            .unwrap_or(self.start_s);
    }

    pub fn text(&self) -> String {
        self.words
            .iter()
            .map(|word| word.text.as_str())
            .collect::<Vec<_>>()
            .join(" ")
    }

    pub fn is_empty(&self) -> bool {
        self.words.iter().all(TranscriptWord::is_empty)
    }
}

/// Complete transcription of the vocal stem.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Transcript {
    /// ISO-639-1 code reported by the engine, e.g. `es` or `en`.
    pub language: String,
    /// Probability of the detected language in `0..=1`.
    pub language_confidence: f32,
    pub lines: Vec<TranscriptLine>,
    /// Segments removed by the hallucination filter.
    pub discarded_segments: usize,
}

impl Transcript {
    pub fn words(&self) -> impl Iterator<Item = &TranscriptWord> {
        self.lines.iter().flat_map(|line| line.words.iter())
    }

    pub fn word_count(&self) -> usize {
        self.lines.iter().map(|line| line.words.len()).sum()
    }

    pub fn is_empty(&self) -> bool {
        self.lines.iter().all(TranscriptLine::is_empty)
    }

    pub fn text(&self) -> String {
        self.lines
            .iter()
            .map(TranscriptLine::text)
            .collect::<Vec<_>>()
            .join("\n")
    }

    pub fn duration_s(&self) -> f64 {
        self.lines.last().map(|line| line.end_s).unwrap_or_default()
    }

    pub fn words_needing_syllables(&self) -> usize {
        self.words().filter(|word| word.needs_syllables()).count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn word(text: &str, start: f64, end: f64) -> TranscriptWord {
        TranscriptWord {
            text: text.into(),
            start_s: start,
            end_s: end,
            confidence: 0.9,
            ..TranscriptWord::default()
        }
    }

    #[test]
    fn line_bounds_follow_the_words() {
        let mut line = TranscriptLine::new(vec![word("hola", 1.0, 1.5), word("mundo", 1.6, 2.2)]);
        assert_eq!(line.start_s, 1.0);
        assert_eq!(line.end_s, 2.2);
        line.words.remove(1);
        line.refresh_bounds();
        assert_eq!(line.end_s, 1.5);
    }

    #[test]
    fn editing_a_word_clears_only_its_syllables() {
        let mut transcript = Transcript {
            language: "es".into(),
            language_confidence: 0.99,
            lines: vec![TranscriptLine::new(vec![
                TranscriptWord {
                    syllables: vec!["ho".into(), "la".into()],
                    ..word("hola", 0.0, 0.4)
                },
                TranscriptWord {
                    syllables: vec!["mun".into(), "do".into()],
                    ..word("mundo", 0.5, 1.0)
                },
            ])],
            discarded_segments: 0,
        };

        transcript.lines[0].words[1].set_text("mondo");
        assert_eq!(transcript.lines[0].words[0].syllables, vec!["ho", "la"]);
        assert!(transcript.lines[0].words[1].syllables.is_empty());
        assert_eq!(transcript.words_needing_syllables(), 1);
        assert_eq!(transcript.word_count(), 2);
        assert_eq!(transcript.text(), "hola mondo");
    }

    #[test]
    fn transcript_round_trips_through_json() {
        let transcript = Transcript {
            language: "en".into(),
            language_confidence: 0.87,
            lines: vec![TranscriptLine::new(vec![word("don't", 0.0, 0.3)])],
            discarded_segments: 2,
        };
        let json = serde_json::to_string(&transcript).unwrap();
        let parsed: Transcript = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, transcript);
        assert_eq!(parsed.duration_s(), 0.3);
    }
}

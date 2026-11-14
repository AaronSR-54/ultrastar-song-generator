//! Syllable splitting for Spanish and English.
//!
//! Both dictionaries (Knuth-Liang patterns) are embedded in the binary, so the
//! app never reads files or network to split a word. Spanish output is refined
//! with explicit diphthong/triphthong/hiatus rules because the pattern set
//! alone sometimes breaks a diphthong or joins two strong vowels.
//!
//! Spanish and English splits are orthographic, not phonetic: they are a good
//! first approximation for singing and the editor covers the doubtful cases.

use std::sync::OnceLock;

use hyphenation::{Hyphenator, Iter, Language, Load, Standard};

use crate::transcript_edit::{Syllabifier, SyllableSplit};

/// Languages with an embedded dictionary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DictLanguage {
    Spanish,
    English,
}

impl DictLanguage {
    /// Maps an ISO-639-1 code or a Whisper language name to a dictionary.
    pub fn from_code(language: &str) -> Option<Self> {
        let normalized = language.trim().to_lowercase();
        if normalized.starts_with("es") || normalized.contains("spanish") {
            Some(Self::Spanish)
        } else if normalized.starts_with("en") || normalized.contains("english") {
            Some(Self::English)
        } else {
            None
        }
    }
}

/// Hyphenation dictionaries loaded once and reused.
pub struct DictionarySyllabifier {
    spanish: Option<Standard>,
    english: Option<Standard>,
}

impl DictionarySyllabifier {
    pub fn new() -> Self {
        Self {
            spanish: Standard::from_embedded(Language::Spanish).ok(),
            english: Standard::from_embedded(Language::EnglishUS).ok(),
        }
    }

    pub fn has_dictionary(&self, language: DictLanguage) -> bool {
        match language {
            DictLanguage::Spanish => self.spanish.is_some(),
            DictLanguage::English => self.english.is_some(),
        }
    }

    fn dictionary(&self, language: DictLanguage) -> Option<&Standard> {
        match language {
            DictLanguage::Spanish => self.spanish.as_ref(),
            DictLanguage::English => self.english.as_ref(),
        }
    }

    /// Splits with the dictionary for `language`, or with the generic fallback.
    pub fn split_word(&self, language: &str, word: &str) -> SyllableSplit {
        let word = word.trim();
        if word.is_empty() {
            return SyllableSplit {
                syllables: Vec::new(),
                known: true,
            };
        }

        // Contractions and hyphenated compounds are kept whole: splitting
        // "don't" would create an empty syllable, and UltraStar joins syllables
        // without spaces, so a wrong split would change the lyrics.
        if word.contains('\'') || word.contains('’') || word.contains('-') {
            return SyllableSplit {
                syllables: vec![word.to_string()],
                known: true,
            };
        }

        let Some(language) = DictLanguage::from_code(language) else {
            return SyllableSplit {
                syllables: fallback_split(word),
                known: false,
            };
        };
        if language == DictLanguage::English {
            if let Some(syllables) = english_exception(word) {
                return SyllableSplit {
                    syllables,
                    known: true,
                };
            }
        }
        let Some(dictionary) = self.dictionary(language) else {
            return SyllableSplit {
                syllables: fallback_split(word),
                known: false,
            };
        };

        let mut syllables: Vec<String> = dictionary
            .hyphenate(word)
            .iter()
            .segments()
            .map(str::to_string)
            .collect();
        syllables.retain(|syllable| !syllable.is_empty());

        if syllables.is_empty() {
            syllables.push(word.to_string());
        }
        if language == DictLanguage::Spanish {
            syllables = apply_spanish_rules(syllables);
        }

        SyllableSplit {
            syllables,
            known: true,
        }
    }
}

impl Default for DictionarySyllabifier {
    fn default() -> Self {
        Self::new()
    }
}

impl Syllabifier for DictionarySyllabifier {
    fn syllables(&self, language: &str, word: &str) -> Vec<String> {
        self.split_word(language, word).syllables
    }

    fn split(&self, language: &str, word: &str) -> SyllableSplit {
        self.split_word(language, word)
    }
}

/// Process-wide instance: deserialising the dictionaries is not free and the
/// data is immutable.
pub fn dictionary_syllabifier() -> &'static DictionarySyllabifier {
    static INSTANCE: OnceLock<DictionarySyllabifier> = OnceLock::new();
    INSTANCE.get_or_init(DictionarySyllabifier::new)
}

/// Common lyric words the Knuth-Liang English patterns do not split (they are
/// too short or appear as exceptions). Splitting them makes two-syllable words
/// line up with two notes instead of forcing a doubtful grouping.
const ENGLISH_EXCEPTIONS: &[(&str, &[&str])] = &[
    ("every", &["ev", "ery"]),
    ("never", &["nev", "er"]),
    ("ever", &["ev", "er"]),
    ("even", &["e", "ven"]),
    ("over", &["o", "ver"]),
    ("seven", &["sev", "en"]),
    ("heaven", &["heav", "en"]),
    ("forever", &["for", "ev", "er"]),
    ("another", &["an", "oth", "er"]),
    ("because", &["be", "cause"]),
];

fn english_exception(word: &str) -> Option<Vec<String>> {
    let lowercase = word.to_lowercase();
    let (_, parts) = ENGLISH_EXCEPTIONS
        .iter()
        .find(|(key, _)| *key == lowercase)?;
    let chars: Vec<char> = word.chars().collect();
    let mut result = Vec::with_capacity(parts.len());
    let mut consumed = 0;
    for part in *parts {
        let end = consumed + part.chars().count();
        if end > chars.len() {
            return None;
        }
        result.push(chars[consumed..end].iter().collect());
        consumed = end;
    }
    (consumed == chars.len()).then_some(result)
}

/// True when the character is one of the Spanish weak vowels.
fn is_weak(c: char) -> bool {
    matches!(c, 'i' | 'u' | 'ü')
}

/// Weak vowels carrying a written accent break the diphthong (`pa-ís`).
fn is_accented_weak(c: char) -> bool {
    matches!(c, 'í' | 'ú' | 'ì' | 'ù')
}

fn is_strong(c: char) -> bool {
    matches!(c, 'a' | 'e' | 'o' | 'á' | 'é' | 'ó')
}

fn is_vowel(c: char) -> bool {
    is_weak(c) || is_strong(c)
}

/// Whether two adjacent syllables form a diphthong and must be joined.
fn is_diphthong_boundary(left: &str, right: &str) -> bool {
    let Some(l) = left.chars().last() else {
        return false;
    };
    let Some(r) = right.chars().next() else {
        return false;
    };
    if is_accented_weak(l) || is_accented_weak(r) {
        return false;
    }
    let left_weak = is_weak(l);
    let right_weak = is_weak(r);
    let left_strong = is_strong(l);
    let right_strong = is_strong(r);

    if !(is_vowel(l) && is_vowel(r)) {
        return false;
    }
    // weak+weak, weak+strong and strong+weak are diphthongs; strong+strong is a
    // hiatus and must stay split.
    (left_weak && right_weak) || (left_weak && right_strong) || (left_strong && right_weak)
}

/// Joins syllables that the pattern set split inside a diphthong or triphthong.
pub fn apply_spanish_rules(parts: Vec<String>) -> Vec<String> {
    let mut result: Vec<String> = Vec::with_capacity(parts.len());
    for part in parts {
        if part.is_empty() {
            continue;
        }
        match result.last_mut() {
            Some(previous) if is_diphthong_boundary(previous, &part) => previous.push_str(&part),
            _ => result.push(part),
        }
    }
    if result.is_empty() {
        result.push(String::new());
    }
    result
}

/// Lossless fallback: one syllable per consonant run plus the following vowel
/// run, with trailing consonants attached to the last syllable.
pub fn fallback_split(word: &str) -> Vec<String> {
    let chars: Vec<char> = word.chars().collect();
    let mut syllables: Vec<String> = Vec::new();
    let mut start = 0;
    let mut index = 0;

    while index < chars.len() {
        let mut cursor = index;
        while cursor < chars.len() && !is_vowel(chars[cursor]) {
            cursor += 1;
        }
        if cursor == chars.len() {
            break;
        }
        while cursor < chars.len() && is_vowel(chars[cursor]) {
            cursor += 1;
        }
        if cursor > start {
            syllables.push(chars[start..cursor].iter().collect());
            start = cursor;
        }
        index = cursor;
    }

    if start < chars.len() {
        match syllables.last_mut() {
            Some(last) => last.extend(chars[start..].iter()),
            None => syllables.push(chars[start..].iter().collect()),
        }
    }
    if syllables.is_empty() {
        syllables.push(word.to_string());
    }
    syllables
}

#[cfg(test)]
mod tests {
    use super::*;

    fn split(language: &str, word: &str) -> SyllableSplit {
        dictionary_syllabifier().split_word(language, word)
    }

    #[test]
    fn spanish_diphthong_stays_together() {
        let result = split("es", "canción");
        assert_eq!(result.syllables, vec!["can", "ción"]);
        assert!(result.known);
    }

    #[test]
    fn english_contraction_is_a_single_unit() {
        let result = split("en", "don't");
        assert_eq!(result.syllables, vec!["don't"]);
        assert!(result.known);
        assert!(result.syllables.iter().all(|syllable| !syllable.is_empty()));
    }

    #[test]
    fn english_word_is_split_orthographically() {
        let result = split("en", "every");
        assert_eq!(result.syllables, vec!["ev", "ery"]);
        assert!(result.known);
        assert_eq!(result.syllables.concat(), "every");

        let capitalized = split("en", "Every");
        assert_eq!(capitalized.syllables, vec!["Ev", "ery"]);
    }

    #[test]
    fn unknown_language_uses_the_fallback_and_is_flagged() {
        let result = split("xx", "brzzk");
        assert!(!result.known);
        assert!(!result.syllables.is_empty());
        assert_eq!(result.syllables.concat(), "brzzk");
        assert!(result.syllables.iter().all(|syllable| !syllable.is_empty()));
    }

    #[test]
    fn spanish_rules_join_diphthongs_and_keep_hiatuses() {
        assert_eq!(
            apply_spanish_rules(vec!["ci".into(), "u".into(), "dad".into()]),
            vec!["ciu", "dad"]
        );
        assert_eq!(
            apply_spanish_rules(vec!["pa".into(), "í".into(), "s".into()]),
            vec!["pa", "í", "s"]
        );
        assert_eq!(
            apply_spanish_rules(vec!["ca".into(), "os".into()]),
            vec!["ca", "os"]
        );
        assert_eq!(
            apply_spanish_rules(vec!["bue".into(), "no".into()]),
            vec!["bue", "no"]
        );
    }

    #[test]
    fn splitting_is_lossless_and_never_empty() {
        for (language, word) in [
            ("es", "canción"),
            ("es", "ciudad"),
            ("es", "murciélago"),
            ("es", "güero"),
            ("en", "every"),
            ("en", "hyphenation"),
            ("en", "rhythm"),
            ("xx", "xyzzy"),
        ] {
            let result = split(language, word);
            assert!(
                result.syllables.iter().all(|syllable| !syllable.is_empty()),
                "{word}: {:?}",
                result.syllables
            );
            assert_eq!(result.syllables.concat(), word, "{language} {word}");
        }
    }

    #[test]
    fn dictionary_language_codes_are_recognised() {
        assert_eq!(DictLanguage::from_code("es"), Some(DictLanguage::Spanish));
        assert_eq!(
            DictLanguage::from_code("es-ES"),
            Some(DictLanguage::Spanish)
        );
        assert_eq!(DictLanguage::from_code("en"), Some(DictLanguage::English));
        assert_eq!(
            DictLanguage::from_code("english"),
            Some(DictLanguage::English)
        );
        assert_eq!(DictLanguage::from_code("fr"), None);
    }
}

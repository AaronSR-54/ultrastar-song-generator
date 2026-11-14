use std::fmt;
use std::path::Path;

use usdx_domain::{Beat, Note, NoteKind, Phrase, PhraseBreak, Pitch, SongDoc, SongHeader};

/// Non-fatal issue found while reading a file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseWarning {
    pub line: usize,
    pub message: String,
}

impl fmt::Display for ParseWarning {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.line == 0 {
            write!(f, "{}", self.message)
        } else {
            write!(f, "line {}: {}", self.line, self.message)
        }
    }
}

/// Result of a tolerant parse: the document plus every issue found.
#[derive(Debug, Clone, PartialEq)]
pub struct ParseResult {
    pub doc: SongDoc,
    pub warnings: Vec<ParseWarning>,
}

impl ParseResult {
    pub fn has_warnings(&self) -> bool {
        !self.warnings.is_empty()
    }
}

/// Parses an UltraStar `.txt` document.
///
/// The parser never aborts on a malformed line: it records a warning and keeps
/// reading the rest of the file. Unknown headers are preserved in
/// [`SongHeader::extra`] so a writer can round-trip them.
pub fn parse(input: &str) -> ParseResult {
    let input = input.strip_prefix('\u{feff}').unwrap_or(input);
    let mut doc = SongDoc::default();
    let mut warnings = Vec::new();
    let mut current = Phrase::default();
    let mut has_current = false;
    let mut player = 1_u8;
    let mut p2_notice = false;

    for (index, raw) in input.lines().enumerate() {
        let line_no = index + 1;
        let line = raw.trim_end_matches('\r').trim();
        if line.is_empty() {
            continue;
        }

        if line.eq_ignore_ascii_case("E") {
            doc.has_end_marker = true;
            continue;
        }
        if line.eq_ignore_ascii_case("P1") {
            player = 1;
            continue;
        }
        if line.eq_ignore_ascii_case("P2") {
            player = 2;
            doc.is_duet = true;
            close_phrase(&mut current, &mut has_current, &mut doc);
            continue;
        }

        if let Some(rest) = line.strip_prefix('#') {
            parse_header(rest, line_no, &mut doc, &mut warnings);
            continue;
        }

        if let Some(rest) = line.strip_prefix('-') {
            if let Some(break_marker) = parse_break(rest, line_no, &mut warnings) {
                current.break_after = Some(break_marker);
                close_phrase(&mut current, &mut has_current, &mut doc);
            }
            continue;
        }

        let Some(marker) = line.chars().next() else {
            continue;
        };

        if let Some(kind) = note_kind(marker) {
            if player == 2 {
                if !p2_notice {
                    warnings.push(ParseWarning {
                        line: line_no,
                        message: "second player (P2) notes are ignored in the MVP".into(),
                    });
                    p2_notice = true;
                }
                continue;
            }
            if let Some(note) = parse_note(kind, &line[marker.len_utf8()..], line_no, &mut warnings)
            {
                current.notes.push(note);
                has_current = true;
            }
            continue;
        }

        warnings.push(ParseWarning {
            line: line_no,
            message: format!("unrecognised line: {line}"),
        });
    }

    close_phrase(&mut current, &mut has_current, &mut doc);

    ParseResult { doc, warnings }
}

/// Reads a file as UTF-8, falling back to a lossy conversion for legacy encodings.
pub fn parse_file(path: impl AsRef<Path>) -> std::io::Result<ParseResult> {
    let bytes = std::fs::read(path)?;
    Ok(parse_bytes(&bytes))
}

/// Parses raw bytes, reporting when the source was not valid UTF-8.
pub fn parse_bytes(bytes: &[u8]) -> ParseResult {
    match std::str::from_utf8(bytes) {
        Ok(text) => parse(text),
        Err(_) => {
            let mut result = parse(&String::from_utf8_lossy(bytes));
            result.warnings.push(ParseWarning {
                line: 0,
                message: "file is not valid UTF-8; invalid bytes were replaced".into(),
            });
            result
        }
    }
}

fn close_phrase(current: &mut Phrase, has_current: &mut bool, doc: &mut SongDoc) {
    if *has_current || !current.notes.is_empty() || current.break_after.is_some() {
        doc.phrases.push(std::mem::take(current));
    }
    *has_current = false;
}

fn note_kind(marker: char) -> Option<NoteKind> {
    match marker {
        ':' => Some(NoteKind::Normal),
        '*' => Some(NoteKind::Golden),
        'F' | 'f' => Some(NoteKind::Freestyle),
        'R' | 'r' => Some(NoteKind::Rap),
        'G' | 'g' => Some(NoteKind::RapGolden),
        _ => None,
    }
}

fn parse_note(
    kind: NoteKind,
    rest: &str,
    line_no: usize,
    warnings: &mut Vec<ParseWarning>,
) -> Option<Note> {
    let (start, rest) = next_token(rest)?;
    let (length, rest) = next_token(rest)?;
    let (pitch, rest) = next_token(rest)?;

    let start_beat = match parse_beat(start) {
        Some(value) => value,
        None => {
            warnings.push(ParseWarning {
                line: line_no,
                message: format!("invalid note start beat: {start}"),
            });
            return None;
        }
    };
    let mut length_beats = match parse_beat(length) {
        Some(value) => value,
        None => {
            warnings.push(ParseWarning {
                line: line_no,
                message: format!("invalid note length: {length}"),
            });
            return None;
        }
    };
    if length_beats == 0 {
        warnings.push(ParseWarning {
            line: line_no,
            message: "note length 0 clamped to 1".into(),
        });
        length_beats = 1;
    }
    let pitch: Pitch = match pitch.parse() {
        Ok(value) => value,
        Err(_) => {
            warnings.push(ParseWarning {
                line: line_no,
                message: format!("invalid note pitch: {pitch}"),
            });
            return None;
        }
    };

    let (text, starts_word) = split_text(rest);
    // A continuation carries no word-start meaning; normalise the flag so a
    // parsed file and a re-parsed written file compare equal.
    let starts_word = starts_word && text.trim() != "~";

    Some(Note {
        kind,
        start_beat,
        length_beats,
        pitch,
        text,
        starts_word,
        confidence: None,
    })
}

/// Extracts the lyric text after the pitch token.
///
/// The format uses exactly one space between pitch and text; a second leading
/// space marks the start of a new word.
fn split_text(rest: &str) -> (String, bool) {
    let without_separator = rest
        .strip_prefix(' ')
        .or_else(|| rest.strip_prefix('\t'))
        .unwrap_or(rest);
    if let Some(text) = without_separator.strip_prefix(' ') {
        (text.to_string(), true)
    } else {
        (without_separator.to_string(), false)
    }
}

fn next_token(input: &str) -> Option<(&str, &str)> {
    let trimmed = input.trim_start_matches(char::is_whitespace);
    if trimmed.is_empty() {
        return None;
    }
    let end = trimmed.find(char::is_whitespace).unwrap_or(trimmed.len());
    Some((&trimmed[..end], &trimmed[end..]))
}

fn parse_beat(value: &str) -> Option<Beat> {
    value.parse::<Beat>().ok()
}

fn parse_break(
    rest: &str,
    line_no: usize,
    warnings: &mut Vec<ParseWarning>,
) -> Option<PhraseBreak> {
    let mut tokens = rest.split_whitespace();
    let start = tokens.next()?;
    let start_beat = match parse_beat(start) {
        Some(value) => value,
        None => {
            warnings.push(ParseWarning {
                line: line_no,
                message: format!("invalid phrase break beat: {start}"),
            });
            return None;
        }
    };
    let end_beat = match tokens.next() {
        Some(value) => match parse_beat(value) {
            Some(value) => Some(value),
            None => {
                warnings.push(ParseWarning {
                    line: line_no,
                    message: format!("invalid phrase break end beat: {value}"),
                });
                None
            }
        },
        None => None,
    };

    if tokens.next().is_some() {
        warnings.push(ParseWarning {
            line: line_no,
            message: "extra tokens after phrase break".into(),
        });
    }

    Some(PhraseBreak {
        start_beat,
        end_beat,
    })
}

fn parse_header(rest: &str, line_no: usize, doc: &mut SongDoc, warnings: &mut Vec<ParseWarning>) {
    let Some((key, raw_value)) = rest.split_once(':') else {
        if rest.trim().eq_ignore_ascii_case("END") {
            doc.has_end_marker = true;
            return;
        }
        warnings.push(ParseWarning {
            line: line_no,
            message: format!("header without value: #{rest}"),
        });
        return;
    };

    let key = key.trim().to_ascii_uppercase();
    let value = raw_value.trim();
    let header: &mut SongHeader = &mut doc.header;

    match key.as_str() {
        "TITLE" => header.title = value.to_string(),
        "ARTIST" => header.artist = value.to_string(),
        "MP3" | "AUDIO" => header.audio = value.to_string(),
        "BPM" => match parse_decimal(value) {
            Some(bpm) if bpm > 0.0 => header.bpm = bpm,
            Some(_) => warnings.push(ParseWarning {
                line: line_no,
                message: format!("BPM must be positive: {value}"),
            }),
            None => warnings.push(ParseWarning {
                line: line_no,
                message: format!("invalid BPM: {value}"),
            }),
        },
        "GAP" => match parse_decimal(value) {
            Some(gap) => header.gap_ms = gap.round() as i64,
            None => warnings.push(ParseWarning {
                line: line_no,
                message: format!("invalid GAP: {value}"),
            }),
        },
        "LANGUAGE" => header.language = non_empty(value),
        "GENRE" => header.genre = non_empty(value),
        "YEAR" => match value.parse::<i32>() {
            Ok(year) => header.year = Some(year),
            Err(_) => warnings.push(ParseWarning {
                line: line_no,
                message: format!("invalid YEAR: {value}"),
            }),
        },
        "CREATOR" => header.creator = non_empty(value),
        "COVER" => header.cover = non_empty(value),
        "VIDEO" => header.video = non_empty(value),
        "VIDEOGAP" => match parse_decimal(value) {
            Some(gap) => header.video_gap_s = Some(gap),
            None => warnings.push(ParseWarning {
                line: line_no,
                message: format!("invalid VIDEOGAP: {value}"),
            }),
        },
        "BACKGROUND" => header.background = non_empty(value),
        "EDITION" => header.edition = non_empty(value),
        "MEDLEYSTARTBEAT" => match parse_beat(value) {
            Some(beat) => header.medley_start_beat = Some(beat),
            None => warnings.push(ParseWarning {
                line: line_no,
                message: format!("invalid MEDLEYSTARTBEAT: {value}"),
            }),
        },
        "MEDLEYENDBEAT" => match parse_beat(value) {
            Some(beat) => header.medley_end_beat = Some(beat),
            None => warnings.push(ParseWarning {
                line: line_no,
                message: format!("invalid MEDLEYENDBEAT: {value}"),
            }),
        },
        "END" => doc.has_end_marker = true,
        _ => {
            header.extra.insert(key.clone(), value.to_string());
            warnings.push(ParseWarning {
                line: line_no,
                message: format!("unknown header: #{key}"),
            });
        }
    }
}

fn parse_decimal(value: &str) -> Option<f64> {
    value.trim().replace(',', ".").parse::<f64>().ok()
}

fn non_empty(value: &str) -> Option<String> {
    (!value.is_empty()).then(|| value.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_bom_crlf_comma_and_unknown_tag() {
        let input = "\u{feff}#TITLE:Song\r\n#artist:Artist\r\n#BPM:313,34\r\n#GAP:-81090\r\n#CUSTOM:keep\r\n: 0 2 -2 Hel\r\n: 3 4 -2 lo\r\n: 8 2 0 ~\r\n- 10\r\n* 12 3 4 gold\r\nE\r\n";
        let result = parse(input);

        assert_eq!(result.doc.header.title, "Song");
        assert_eq!(result.doc.header.artist, "Artist");
        assert!((result.doc.header.bpm - 313.34).abs() < 1e-9);
        assert_eq!(result.doc.header.gap_ms, -81_090);
        assert_eq!(
            result.doc.header.extra.get("CUSTOM").map(String::as_str),
            Some("keep")
        );
        assert_eq!(result.doc.note_count(), 4);
        assert!(result.doc.has_end_marker);
        assert!(!result.doc.is_duet);

        let notes: Vec<_> = result.doc.notes().collect();
        assert_eq!(notes[0].text, "Hel");
        assert!(!notes[0].starts_word);
        assert_eq!(notes[1].text, "lo");
        assert!(!notes[1].starts_word);
        assert!(notes[2].is_continuation());
        assert_eq!(notes[3].kind, NoteKind::Golden);
        assert_eq!(notes[3].start_beat, 12);

        let first = &result.doc.phrases[0];
        assert_eq!(
            first.break_after,
            Some(PhraseBreak {
                start_beat: 10,
                end_beat: None
            })
        );
        assert!(result
            .warnings
            .iter()
            .any(|w| w.message.contains("unknown header: #CUSTOM")));
    }

    #[test]
    fn accepts_dot_decimals_and_lowercase_tags() {
        let result = parse("#bpm: 100.5\n#gap: 250\n: 0 1 0 x\n");
        assert!((result.doc.header.bpm - 100.5).abs() < 1e-9);
        assert_eq!(result.doc.header.gap_ms, 250);
        assert_eq!(result.doc.note_count(), 1);
    }

    #[test]
    fn detects_duet_and_ignores_p2() {
        let result = parse("#TITLE:Duet\nP1\n: 0 1 0 one\nP2\n: 0 1 0 two\nE\n");
        assert!(result.doc.is_duet);
        assert_eq!(result.doc.note_count(), 1);
        assert_eq!(result.doc.notes().next().unwrap().text, "one");
        assert!(result.warnings.iter().any(|w| w.message.contains("P2")));
    }

    #[test]
    fn malformed_lines_warn_without_aborting() {
        let result = parse("#BPM:nope\n: 0 1 0 ok\n: x y z bad\n???\nE\n");
        assert_eq!(result.doc.note_count(), 1);
        assert!(result.doc.has_end_marker);
        assert!(result.warnings.len() >= 3, "{:?}", result.warnings);
    }

    #[test]
    fn phrase_break_with_end_beat() {
        let result = parse(": 0 1 0 x\n- 5 8\n: 8 1 0 y\n");
        assert_eq!(result.doc.phrases.len(), 2);
        assert_eq!(
            result.doc.phrases[0].break_after,
            Some(PhraseBreak {
                start_beat: 5,
                end_beat: Some(8)
            })
        );
    }

    #[test]
    fn note_text_can_contain_spaces() {
        let result = parse(": 0 1 0 hello world\n");
        assert_eq!(result.doc.notes().next().unwrap().text, "hello world");
    }

    #[test]
    fn double_space_marks_the_start_of_a_word() {
        let result = parse(": 0 1 0 first\n: 1 1 0  second\n");
        let notes: Vec<_> = result.doc.notes().collect();
        assert!(!notes[0].starts_word);
        assert!(notes[1].starts_word);
        assert_eq!(notes[1].text, "second");
    }

    #[test]
    fn continuation_ignores_word_start_flag() {
        let result = parse(": 0 2 0 word\n: 2 2 0  ~\n");
        let notes: Vec<_> = result.doc.notes().collect();
        assert!(notes[1].is_continuation());
        assert!(!notes[1].starts_word);
    }

    #[test]
    fn hash_end_and_empty_input() {
        let empty = parse("");
        assert_eq!(empty.doc, SongDoc::default());
        assert!(empty.warnings.is_empty());

        let end = parse("#TITLE:X\n#END\n");
        assert!(end.doc.has_end_marker);
    }

    #[test]
    fn lossy_bytes_report_encoding_warning() {
        let result = parse_bytes(b"#TITLE:\xff\n: 0 1 0 x\n");
        assert_eq!(result.doc.note_count(), 1);
        assert!(result.warnings.iter().any(|w| w.message.contains("UTF-8")));
    }
}

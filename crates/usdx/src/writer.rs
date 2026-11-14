use std::fmt::Write as _;
use std::path::Path;

use usdx_domain::{Note, NoteKind, Phrase, SongDoc, SongHeader};

/// Errors that prevent writing a valid UltraStar file.
#[derive(Debug, thiserror::Error)]
pub enum WriteError {
    #[error("duet songs (P1/P2) are not supported in the MVP")]
    DuetsUnsupported,
    #[error("i/o error: {0}")]
    Io(#[from] std::io::Error),
}

/// Serializes a song into the deterministic UltraStar `.txt` representation.
pub fn write(doc: &SongDoc) -> Result<String, WriteError> {
    if doc.is_duet {
        return Err(WriteError::DuetsUnsupported);
    }

    let mut output = String::new();
    write_header(&mut output, &doc.header);

    for phrase in &doc.phrases {
        write_phrase(&mut output, phrase);
    }

    output.push_str("E\n");
    Ok(output)
}

/// Writes the song to a file as UTF-8 without BOM and with LF line endings.
pub fn write_file(path: impl AsRef<Path>, doc: &SongDoc) -> Result<(), WriteError> {
    let contents = write(doc)?;
    std::fs::write(path, contents)?;
    Ok(())
}

fn write_header(output: &mut String, header: &SongHeader) {
    push_header(output, "TITLE", &header.title);
    push_header(output, "ARTIST", &header.artist);
    if let Some(value) = &header.language {
        push_header(output, "LANGUAGE", value);
    }
    if let Some(value) = &header.genre {
        push_header(output, "GENRE", value);
    }
    if let Some(value) = header.year {
        push_header(output, "YEAR", &value.to_string());
    }
    if let Some(value) = &header.creator {
        push_header(output, "CREATOR", value);
    }
    push_header(output, "MP3", &header.audio);
    if let Some(value) = &header.cover {
        push_header(output, "COVER", value);
    }
    if let Some(value) = &header.video {
        push_header(output, "VIDEO", value);
    }
    if let Some(value) = header.video_gap_s {
        push_header(output, "VIDEOGAP", &format_decimal(value));
    }
    if let Some(value) = &header.background {
        push_header(output, "BACKGROUND", value);
    }
    if let Some(value) = &header.edition {
        push_header(output, "EDITION", value);
    }
    push_header(output, "BPM", &format_decimal(header.bpm));
    push_header(output, "GAP", &header.gap_ms.to_string());
    if let Some(value) = header.medley_start_beat {
        push_header(output, "MEDLEYSTARTBEAT", &value.to_string());
    }
    if let Some(value) = header.medley_end_beat {
        push_header(output, "MEDLEYENDBEAT", &value.to_string());
    }

    for (key, value) in &header.extra {
        push_header(output, key, value);
    }
}

fn push_header(output: &mut String, key: &str, value: &str) {
    let _ = writeln!(output, "#{key}:{}", sanitize(value));
}

fn write_phrase(output: &mut String, phrase: &Phrase) {
    for note in &phrase.notes {
        write_note(output, note);
    }

    if let Some(break_marker) = &phrase.break_after {
        match break_marker.end_beat {
            Some(end) => {
                let _ = writeln!(output, "- {} {}", break_marker.start_beat, end);
            }
            None => {
                let _ = writeln!(output, "- {}", break_marker.start_beat);
            }
        }
    }
}

fn write_note(output: &mut String, note: &Note) {
    let marker = match note.kind {
        NoteKind::Normal => ':',
        NoteKind::Golden => '*',
        NoteKind::Freestyle => 'F',
        NoteKind::Rap => 'R',
        NoteKind::RapGolden => 'G',
    };

    let text = if note.is_continuation() {
        "~".to_string()
    } else if note.starts_word {
        format!(" {}", sanitize(&note.text))
    } else {
        sanitize(&note.text)
    };

    let _ = writeln!(
        output,
        "{} {} {} {} {}",
        marker, note.start_beat, note.length_beats, note.pitch, text
    );
}

fn sanitize(value: &str) -> String {
    value.replace(['\r', '\n'], " ").trim_end().to_string()
}

/// Formats a decimal with a comma separator, preserving the shortest
/// representation that round-trips through `f64`.
pub fn format_decimal(value: f64) -> String {
    if !value.is_finite() {
        return value.to_string().replace('.', ",");
    }

    let mut formatted = value.to_string();
    if formatted.contains(['e', 'E']) {
        formatted = format!("{value:.12}");
        if formatted.contains('.') {
            formatted = formatted
                .trim_end_matches('0')
                .trim_end_matches('.')
                .to_string();
        }
    }
    formatted.replace('.', ",")
}

#[cfg(test)]
mod tests {
    use super::*;
    use usdx_domain::PhraseBreak;

    fn sample_doc() -> SongDoc {
        SongDoc {
            header: SongHeader {
                title: "Song".into(),
                artist: "Artist".into(),
                audio: "audio.mp3".into(),
                bpm: 313.34,
                gap_ms: 81_090,
                language: Some("Spanish".into()),
                cover: Some("cover.jpg".into()),
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
                        end_beat: None,
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
    fn writes_expected_snapshot() {
        let expected = "#TITLE:Song\n\
#ARTIST:Artist\n\
#LANGUAGE:Spanish\n\
#MP3:audio.mp3\n\
#COVER:cover.jpg\n\
#BPM:313,34\n\
#GAP:81090\n\
: 0 2 -2 Hel\n\
: 3 4 -2 lo\n\
: 8 2 0 ~\n\
- 10\n\
* 12 3 4  gold\n\
E\n";
        assert_eq!(write(&sample_doc()).unwrap(), expected);
    }

    #[test]
    fn decimal_formatting_is_stable() {
        assert_eq!(format_decimal(313.34), "313,34");
        assert_eq!(format_decimal(100.0), "100");
        assert_eq!(format_decimal(100.5), "100,5");
        assert_eq!(format_decimal(0.25), "0,25");
    }

    #[test]
    fn round_trip_is_semantically_equal() {
        let doc = sample_doc();
        let text = write(&doc).unwrap();
        let parsed = crate::parse(&text);
        assert_eq!(parsed.doc, doc);
        assert!(parsed.warnings.is_empty(), "{:?}", parsed.warnings);
    }

    #[test]
    fn round_trip_preserves_extra_headers() {
        let mut doc = sample_doc();
        doc.header.extra.insert("FIXER".into(), "someone".into());
        let text = write(&doc).unwrap();
        let parsed = crate::parse(&text);
        assert_eq!(
            parsed.doc.header.extra.get("FIXER").map(String::as_str),
            Some("someone")
        );
    }

    #[test]
    fn duets_are_rejected() {
        let mut doc = sample_doc();
        doc.is_duet = true;
        assert!(matches!(write(&doc), Err(WriteError::DuetsUnsupported)));
    }

    #[test]
    fn newlines_in_text_are_sanitized() {
        let mut doc = sample_doc();
        doc.phrases[0].notes[0].text = "Hel\nlo".into();
        let text = write(&doc).unwrap();
        assert!(text.contains(": 0 2 -2 Hel lo\n"));
        assert_eq!(text.lines().filter(|line| line.starts_with(':')).count(), 3);
    }

    #[test]
    fn phrase_break_with_end_beat_is_written() {
        let mut doc = sample_doc();
        doc.phrases[0].break_after = Some(PhraseBreak {
            start_beat: 10,
            end_beat: Some(12),
        });
        let text = write(&doc).unwrap();
        assert!(text.contains("- 10 12\n"));
    }
}

use usdx_domain::NoteKind;
use usdx_format::parse;

#[test]
fn documented_example_parses() {
    let document = include_str!("../../../docs/usdx-format.md");
    let block = document
        .split("```txt")
        .nth(1)
        .expect("documented txt block")
        .split("```")
        .next()
        .expect("closing fence");

    let result = parse(block);
    assert_eq!(result.doc.header.title, "My Song");
    assert_eq!(result.doc.header.artist, "Artist");
    assert_eq!(result.doc.header.bpm, 120.0);
    assert_eq!(result.doc.header.gap_ms, 1000);
    assert_eq!(result.doc.note_count(), 5);
    assert!(result.doc.has_end_marker);
    assert!(!result.doc.is_duet);
    assert!(result.warnings.is_empty(), "{:?}", result.warnings);

    let notes: Vec<_> = result.doc.notes().collect();
    assert_eq!(notes[0].text, "Hel");
    assert_eq!(notes[2].text, "world");
    assert!(notes[2].starts_word);
    assert!(notes[3].is_continuation());
    assert_eq!(notes[4].kind, NoteKind::Golden);
    assert_eq!(
        result.doc.phrases[0]
            .break_after
            .as_ref()
            .map(|b| b.start_beat),
        Some(10)
    );
}

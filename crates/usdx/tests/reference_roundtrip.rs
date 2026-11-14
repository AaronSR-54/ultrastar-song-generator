use std::path::{Path, PathBuf};

use usdx_format::{parse, parse_file, write};

fn collect_txt_files(root: &Path, files: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(root) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_txt_files(&path, files);
        } else if path
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("txt"))
        {
            files.push(path);
        }
    }
}

#[test]
fn reference_songs_parse_and_single_voice_songs_round_trip() {
    let Some(root) = std::env::var_os("USDX_REFS_DIR").map(PathBuf::from) else {
        eprintln!("USDX_REFS_DIR is not set; skipping reference fixture test");
        return;
    };

    let mut files = Vec::new();
    collect_txt_files(&root, &mut files);
    files.sort();

    assert!(
        files.len() >= 132,
        "expected at least 132 reference files under {}, found {}",
        root.display(),
        files.len()
    );

    let mut total_notes = 0_usize;
    let mut single_voice = 0_usize;
    let mut duets = 0_usize;

    for path in &files {
        let result = parse_file(path)
            .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()));
        let mut doc = result.doc;

        assert!(
            doc.note_count() > 0,
            "{} parsed without notes",
            path.display()
        );
        total_notes += doc.note_count();

        if doc.is_duet {
            duets += 1;
            continue;
        }

        single_voice += 1;
        doc.has_end_marker = true;
        let text = write(&doc)
            .unwrap_or_else(|error| panic!("failed to write {}: {error}", path.display()));
        let reparsed = parse(&text);
        assert_eq!(
            reparsed.doc,
            doc,
            "semantic round-trip mismatch for {}",
            path.display()
        );
    }

    eprintln!(
        "parsed {} files ({} single-voice, {} duets), {} notes",
        files.len(),
        single_voice,
        duets,
        total_notes
    );
}

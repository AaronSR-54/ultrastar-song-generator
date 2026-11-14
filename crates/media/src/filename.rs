use std::path::{Path, PathBuf};

const WINDOWS_RESERVED: [&str; 22] = [
    "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
    "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
];

/// Makes a title safe for Windows, macOS and Linux filesystems while keeping
/// Unicode letters and accents.
pub fn sanitize_filename(input: &str) -> String {
    let mut cleaned = String::with_capacity(input.len());
    for character in input.chars() {
        if character.is_control()
            || matches!(
                character,
                '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*'
            )
        {
            cleaned.push(' ');
        } else {
            cleaned.push(character);
        }
    }

    let mut output = cleaned.split_whitespace().collect::<Vec<_>>().join(" ");
    output = output.trim_matches(['.', ' ']).to_string();

    if output.chars().count() > 120 {
        output = output.chars().take(120).collect();
        output = output.trim_matches(['.', ' ']).to_string();
    }

    if output.is_empty() {
        return "song".into();
    }

    let stem = output
        .split('.')
        .next()
        .unwrap_or_default()
        .to_ascii_uppercase();
    if WINDOWS_RESERVED.contains(&stem.as_str()) {
        output.insert(0, '_');
    }

    output
}

/// Returns a non-existing path in `directory`, appending ` (2)`, ` (3)`...
pub fn unique_path(directory: impl AsRef<Path>, base: &str, extension: &str) -> PathBuf {
    let directory = directory.as_ref();
    let base = sanitize_filename(base);
    let mut candidate = directory.join(format!("{base}.{extension}"));
    let mut counter = 2_u32;
    while candidate.exists() {
        candidate = directory.join(format!("{base} ({counter}).{extension}"));
        counter += 1;
    }
    candidate
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replaces_reserved_and_control_characters() {
        assert_eq!(
            sanitize_filename("A: B/C?D*E\"F<G>H|I"),
            "A B C D E F G H I"
        );
        assert_eq!(sanitize_filename("line\nbreak\ttab"), "line break tab");
    }

    #[test]
    fn keeps_unicode_accents_and_emoji() {
        assert_eq!(
            sanitize_filename("Canción de ñandú 🎵"),
            "Canción de ñandú 🎵"
        );
    }

    #[test]
    fn trims_dots_and_spaces_and_falls_back() {
        assert_eq!(sanitize_filename("  ...  "), "song");
        assert_eq!(sanitize_filename("name. "), "name");
    }

    #[test]
    fn protects_windows_reserved_names() {
        assert_eq!(sanitize_filename("CON"), "_CON");
        assert_eq!(sanitize_filename("nul.txt"), "_nul.txt");
    }

    #[test]
    fn truncates_long_titles_on_character_boundaries() {
        let long = "á".repeat(200);
        let sanitized = sanitize_filename(&long);
        assert_eq!(sanitized.chars().count(), 120);
    }

    #[test]
    fn unique_path_avoids_collisions() {
        let directory = tempfile::tempdir().unwrap();
        let first = unique_path(directory.path(), "Song", "mp3");
        assert_eq!(first.file_name().unwrap(), "Song.mp3");
        std::fs::write(&first, b"x").unwrap();
        let second = unique_path(directory.path(), "Song", "mp3");
        assert_eq!(second.file_name().unwrap(), "Song (2).mp3");
        std::fs::write(&second, b"x").unwrap();
        let third = unique_path(directory.path(), "Song", "mp3");
        assert_eq!(third.file_name().unwrap(), "Song (3).mp3");
    }
}

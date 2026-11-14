use usdx_ml::transcription_capabilities;

#[test]
fn documentation_matches_the_linked_whisper_build() {
    let document = include_str!("../../../docs/transcription.md");
    let capabilities = transcription_capabilities();

    assert_eq!(capabilities.model_id, "whisper-large-v3-turbo-q5");
    assert!(capabilities.runs_locally);
    assert!(
        document.contains(&format!("{} idiomas", capabilities.languages.len())),
        "documented language count is stale: {} languages linked",
        capabilities.languages.len()
    );
    assert!(document.contains("574 041 195 bytes"));
    assert!(document.contains("547 MiB"));
    assert_eq!(capabilities.model_size_bytes, 574_041_195);
    assert!(document.contains("16 kHz"));

    for language in ["es", "en"] {
        assert!(
            capabilities
                .languages
                .iter()
                .any(|entry| entry.code == language),
            "whisper build does not expose {language}"
        );
        assert!(
            document.contains(&format!("| `{language}` |")),
            "documentation does not list {language}"
        );
    }

    assert!(capabilities
        .languages
        .iter()
        .any(|language| language.code == "ja"));
    assert!(!capabilities.languages.is_empty());
}

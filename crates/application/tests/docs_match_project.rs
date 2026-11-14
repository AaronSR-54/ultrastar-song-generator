use serde_json::Value;
use usdx_application::{ProjectSettings, ProjectStore, SourceInfo};

fn documented_json() -> Value {
    let document = include_str!("../../../docs/project-layout.md");
    let block = document
        .split("```json")
        .nth(1)
        .expect("documented json block")
        .split("```")
        .next()
        .expect("closing fence");
    serde_json::from_str(block).expect("documented json is valid")
}

#[test]
fn documented_project_json_matches_a_generated_project() {
    let documented = documented_json();

    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().join("song");
    let source = SourceInfo {
        url: "https://www.youtube.com/watch?v=dQw4w9WgXcQ".into(),
        site: Some("YouTube".into()),
        title: "My Song".into(),
        artist: Some("Artist".into()),
        duration_ms: Some(215_000),
        thumbnail_url: None,
    };
    let (_store, project) =
        ProjectStore::create(&root, source, ProjectSettings::default()).unwrap();
    let generated = serde_json::to_value(&project).unwrap();

    assert_eq!(
        documented.get("schema_version"),
        generated.get("schema_version")
    );

    for section in ["source", "settings", "stages"] {
        let documented_section = documented
            .get(section)
            .and_then(Value::as_object)
            .unwrap_or_else(|| panic!("documented {section} section"));
        let generated_section = generated
            .get(section)
            .and_then(Value::as_object)
            .unwrap_or_else(|| panic!("generated {section} section"));

        let mut documented_keys: Vec<_> = documented_section.keys().collect();
        let mut generated_keys: Vec<_> = generated_section.keys().collect();
        documented_keys.sort();
        generated_keys.sort();
        assert_eq!(documented_keys, generated_keys, "keys differ in {section}");
    }

    let documented_stage = &documented["stages"]["ingest"];
    let generated_stage = &generated["stages"]["ingest"];
    assert_eq!(
        documented_stage.get("status"),
        generated_stage.get("status")
    );
    assert_eq!(documented["stages"].as_object().unwrap().len(), 6);
}

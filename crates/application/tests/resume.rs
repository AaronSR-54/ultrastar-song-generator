use usdx_application::{
    now_unix_ms, ProjectSettings, ProjectStore, SourceInfo, StageId, StageStatus,
};

fn project_source() -> SourceInfo {
    SourceInfo {
        url: "https://example.com/watch?v=abc".into(),
        title: "Resume Test".into(),
        ..SourceInfo::default()
    }
}

#[test]
fn a_failed_project_resumes_from_the_failed_stage() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().join("song");
    let (store, mut project) =
        ProjectStore::create(&root, project_source(), ProjectSettings::default()).unwrap();

    let now = now_unix_ms();
    project.stage_mut(StageId::Ingest).start(now).unwrap();
    project.stage_mut(StageId::Ingest).succeed(now).unwrap();
    project.stage_mut(StageId::Separate).start(now).unwrap();
    project
        .stage_mut(StageId::Separate)
        .fail("out of memory", now)
        .unwrap();
    store.save(&project).unwrap();

    let (_store, reopened) = ProjectStore::open(&root).unwrap();
    assert_eq!(reopened.stage(StageId::Ingest).status, StageStatus::Done);
    assert_eq!(
        reopened.stage(StageId::Separate).status,
        StageStatus::Failed
    );
    assert_eq!(reopened.next_stage(), Some(StageId::Separate));
    assert_eq!(
        reopened.stage(StageId::Separate).error.as_deref(),
        Some("out of memory")
    );
}

#[test]
fn changing_the_language_only_invalidates_transcription_and_later() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().join("song");
    let (store, mut project) =
        ProjectStore::create(&root, project_source(), ProjectSettings::default()).unwrap();

    for stage in StageId::ALL {
        project.stage_mut(stage).status = StageStatus::Done;
    }
    store.save(&project).unwrap();

    project.settings.language = Some("Spanish".into());
    project.invalidate_from(StageId::Transcribe).unwrap();
    store.save(&project).unwrap();

    let (_store, reopened) = ProjectStore::open(&root).unwrap();
    assert_eq!(reopened.stage(StageId::Ingest).status, StageStatus::Done);
    assert_eq!(reopened.stage(StageId::Separate).status, StageStatus::Done);
    assert_eq!(reopened.stage(StageId::Pitch).status, StageStatus::Done);
    assert_eq!(
        reopened.stage(StageId::Transcribe).status,
        StageStatus::Pending
    );
    assert_eq!(reopened.stage(StageId::Align).status, StageStatus::Pending);
    assert_eq!(reopened.stage(StageId::Export).status, StageStatus::Pending);
}

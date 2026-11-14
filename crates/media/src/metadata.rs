use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VideoMetadata {
    pub id: String,
    pub title: String,
    pub artist: Option<String>,
    pub duration_ms: u64,
    pub thumbnail_url: Option<String>,
    pub site: Option<String>,
    pub webpage_url: String,
}

#[derive(Debug, thiserror::Error)]
pub enum MetadataError {
    #[error("yt-dlp returned invalid JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("playlists are not supported; provide a single video URL")]
    Playlist,
    #[error("missing required field: {0}")]
    MissingField(&'static str),
}

/// Parses the JSON produced by `yt-dlp --dump-single-json`.
pub fn parse_yt_dlp_json(input: &str) -> Result<VideoMetadata, MetadataError> {
    let value: serde_json::Value = serde_json::from_str(input)?;

    if value
        .get("entries")
        .is_some_and(|entries| entries.is_array())
    {
        return Err(MetadataError::Playlist);
    }

    let title = value
        .get("title")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|title| !title.is_empty())
        .ok_or(MetadataError::MissingField("title"))?
        .to_string();

    let id = value
        .get("id")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default()
        .to_string();

    let webpage_url = value
        .get("webpage_url")
        .or_else(|| value.get("original_url"))
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default()
        .to_string();

    if id.is_empty() && webpage_url.is_empty() {
        return Err(MetadataError::MissingField("id"));
    }

    let artist = ["artist", "creator", "uploader", "channel"]
        .into_iter()
        .find_map(|key| {
            value
                .get(key)
                .and_then(serde_json::Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_string)
        });

    let duration_ms = value
        .get("duration_ms")
        .and_then(serde_json::Value::as_u64)
        .or_else(|| {
            value
                .get("duration")
                .and_then(serde_json::Value::as_f64)
                .map(|seconds| (seconds * 1000.0).round() as u64)
        })
        .unwrap_or(0);

    let thumbnail_url = value
        .get("thumbnail")
        .and_then(serde_json::Value::as_str)
        .map(str::to_string)
        .or_else(|| {
            value
                .get("thumbnails")
                .and_then(serde_json::Value::as_array)
                .and_then(|thumbnails| {
                    thumbnails.iter().rev().find_map(|thumbnail| {
                        thumbnail.get("url").and_then(serde_json::Value::as_str)
                    })
                })
                .map(str::to_string)
        });

    let site = value
        .get("extractor_key")
        .or_else(|| value.get("extractor"))
        .and_then(serde_json::Value::as_str)
        .map(str::to_string);

    Ok(VideoMetadata {
        id,
        title,
        artist,
        duration_ms,
        thumbnail_url,
        site,
        webpage_url,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_a_complete_video() {
        let json = r#"{
            "id": "dQw4w9WgXcQ",
            "title": "Rick Astley - Never Gonna Give You Up",
            "artist": "Rick Astley",
            "duration": 213.5,
            "thumbnail": "https://example.com/thumb.jpg",
            "extractor_key": "Youtube",
            "webpage_url": "https://www.youtube.com/watch?v=dQw4w9WgXcQ"
        }"#;
        let metadata = parse_yt_dlp_json(json).unwrap();
        assert_eq!(metadata.id, "dQw4w9WgXcQ");
        assert_eq!(metadata.title, "Rick Astley - Never Gonna Give You Up");
        assert_eq!(metadata.artist.as_deref(), Some("Rick Astley"));
        assert_eq!(metadata.duration_ms, 213_500);
        assert_eq!(
            metadata.thumbnail_url.as_deref(),
            Some("https://example.com/thumb.jpg")
        );
        assert_eq!(metadata.site.as_deref(), Some("Youtube"));
    }

    #[test]
    fn falls_back_to_uploader_and_thumbnails() {
        let json = r#"{
            "id": "abc",
            "title": "Song",
            "uploader": "Uploader",
            "duration_ms": 1234,
            "thumbnails": [{"url": "a"}, {"url": "b"}]
        }"#;
        let metadata = parse_yt_dlp_json(json).unwrap();
        assert_eq!(metadata.artist.as_deref(), Some("Uploader"));
        assert_eq!(metadata.duration_ms, 1234);
        assert_eq!(metadata.thumbnail_url.as_deref(), Some("b"));
    }

    #[test]
    fn rejects_playlists() {
        let json = r#"{"title": "List", "entries": [{"id": "a"}]}"#;
        assert!(matches!(
            parse_yt_dlp_json(json),
            Err(MetadataError::Playlist)
        ));
    }

    #[test]
    fn rejects_missing_title_and_invalid_json() {
        assert!(matches!(
            parse_yt_dlp_json(r#"{"id": "a"}"#),
            Err(MetadataError::MissingField("title"))
        ));
        assert!(matches!(
            parse_yt_dlp_json("not json"),
            Err(MetadataError::Json(_))
        ));
    }
}

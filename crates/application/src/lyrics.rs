//! Online lyrics lookup (LRCLIB) used before local transcription.
//!
//! LRCLIB is free, needs no API key and returns word-synced lyrics for many
//! tracks. A hit lets the transcribe stage skip Whisper entirely; when nothing
//! usable is found the pipeline falls back to the local model.
//!
//! The API is documented at <https://lrclib.net/docs>. Every record carries a
//! Lyricsfile YAML document, which may include word-level timings.

use std::time::Duration;

use reqwest::StatusCode;
use serde::de::DeserializeOwned;
use serde::Deserialize;
use usdx_domain::{Transcript, TranscriptLine, TranscriptWord};
use usdx_ports::CancelFlag;

const BASE_URL: &str = "https://lrclib.net";
const USER_AGENT: &str = concat!(
    "ultrastar-song-generator/",
    env!("CARGO_PKG_VERSION"),
    " (https://github.com/AaronSR-54/ultrastar-song-generator)"
);
const REQUEST_TIMEOUT: Duration = Duration::from_secs(12);
/// Confidence assigned to word-level timings and to interpolated line timings.
const WORD_SYNC_CONFIDENCE: f32 = 0.9;
const LINE_SYNC_CONFIDENCE: f32 = 0.75;

#[derive(Debug, thiserror::Error)]
pub enum LyricsError {
    #[error("lyrics request failed: {0}")]
    Http(String),
    #[error("lyrics response was invalid: {0}")]
    Invalid(String),
    #[error("lyrics lookup was cancelled")]
    Cancelled,
}

/// What to ask LRCLIB for. `duration_s` improves the exact match.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LyricsQuery {
    pub track_name: String,
    pub artist_name: Option<String>,
    pub duration_s: Option<u64>,
}

impl LyricsQuery {
    /// Builds a clean query from YouTube metadata, which usually carries
    /// decorations ("(Official Video)", "Artist - Song", "… - Topic" channels).
    pub fn from_source(title: &str, artist: Option<&str>, duration_ms: Option<u64>) -> Self {
        let mut track_name = clean_label(title);
        let mut artist_name = artist.map(clean_artist).filter(|value| !value.is_empty());

        if let Some((left, right)) = split_dash(&track_name) {
            let left = left.to_owned();
            let right = right.to_owned();
            let normalized_left = normalize_label(&left);
            let matches_artist = artist_name
                .as_deref()
                .map(|value| normalize_label(value) == normalized_left)
                .unwrap_or(true);
            if matches_artist {
                artist_name = Some(left);
                track_name = right;
            }
        }

        Self {
            track_name,
            artist_name,
            duration_s: duration_ms
                .map(|milliseconds| (milliseconds as f64 / 1000.0).round() as u64),
        }
    }
}

/// One usable lyrics record, already mapped from the API JSON.
#[derive(Debug, Clone, PartialEq)]
pub struct LyricsRecord {
    pub track_name: String,
    pub artist_name: String,
    pub duration_s: f64,
    pub has_word_sync: bool,
    pub synced_lyrics: Option<String>,
    pub plain_lyrics: Option<String>,
    pub lyrics_file: Option<String>,
}

impl LyricsRecord {
    fn is_usable(&self) -> bool {
        self.synced_lyrics.is_some() || self.lyrics_file.is_some()
    }
}

/// Small HTTP client for the public LRCLIB API.
pub struct LyricsClient {
    http: reqwest::Client,
    base_url: String,
}

impl Default for LyricsClient {
    fn default() -> Self {
        Self::new()
    }
}

impl LyricsClient {
    pub fn new() -> Self {
        Self::with_base_url(BASE_URL)
    }

    pub fn with_base_url(base_url: impl Into<String>) -> Self {
        let http = reqwest::Client::builder()
            .user_agent(USER_AGENT)
            .timeout(REQUEST_TIMEOUT)
            .build()
            .expect("failed to build the lyrics HTTP client");
        Self {
            http,
            base_url: base_url.into(),
        }
    }

    /// Looks the track up by signature and, when that fails, by search.
    ///
    /// Returns `Ok(None)` when no record with timings exists; network problems
    /// are errors so the caller can fall back to transcription.
    pub async fn lookup(
        &self,
        query: &LyricsQuery,
        cancel: Option<&CancelFlag>,
    ) -> Result<Option<LyricsRecord>, LyricsError> {
        let track_name = query.track_name.trim();
        if track_name.is_empty() {
            return Ok(None);
        }
        check_cancelled(cancel)?;

        if let (Some(artist), Some(duration)) = (
            query
                .artist_name
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty()),
            query.duration_s.filter(|value| (1..=3600).contains(value)),
        ) {
            let record = self
                .send_json::<ApiRecord>(
                    "/api/get",
                    &[
                        ("track_name", track_name.to_owned()),
                        ("artist_name", artist.to_owned()),
                        ("duration", duration.to_string()),
                    ],
                )
                .await?;
            if let Some(record) = record {
                let record = LyricsRecord::from(record);
                if record.is_usable() {
                    return Ok(Some(record));
                }
            }
        }

        check_cancelled(cancel)?;
        let mut params: Vec<(&str, String)> = vec![("track_name", track_name.to_owned())];
        if let Some(artist) = query
            .artist_name
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            params.push(("artist_name", artist.to_owned()));
        }
        let records = self
            .send_json::<Vec<ApiRecord>>("/api/search", &params)
            .await?
            .unwrap_or_default();
        check_cancelled(cancel)?;
        Ok(best_record(records, query.duration_s))
    }

    async fn send_json<T: DeserializeOwned>(
        &self,
        path: &str,
        params: &[(&str, String)],
    ) -> Result<Option<T>, LyricsError> {
        let mut url = reqwest::Url::parse(&format!("{}{}", self.base_url, path))
            .map_err(|error| LyricsError::Http(error.to_string()))?;
        {
            let mut pairs = url.query_pairs_mut();
            for (key, value) in params {
                pairs.append_pair(key, value);
            }
        }
        let response = self
            .http
            .get(url)
            .send()
            .await
            .map_err(|error| LyricsError::Http(error.to_string()))?;
        let status = response.status();
        if status == StatusCode::NOT_FOUND {
            return Ok(None);
        }
        if !status.is_success() {
            return Err(LyricsError::Http(format!("LRCLIB returned {status}")));
        }
        response
            .json::<T>()
            .await
            .map(Some)
            .map_err(|error| LyricsError::Invalid(error.to_string()))
    }
}

/// Non-musical decoration commonly added to YouTube titles.
const NOISE_PHRASES: &[&str] = &[
    "official video",
    "official music video",
    "official lyric video",
    "official audio",
    "video oficial",
    "videoclip oficial",
    "video musical",
    "lyric video",
    "video con letra",
    "audio oficial",
    "visualizer",
    "audio",
];
const NOISE_TOKENS: &[&str] = &[
    "hd",
    "hq",
    "4k",
    "remastered",
    "remaster",
    "lyrics",
    "letra",
];

fn clean_label(value: &str) -> String {
    let value = strip_noise_segments(value);
    let value = value
        .split('|')
        .filter(|segment| !contains_noise(segment))
        .collect::<Vec<_>>()
        .join(" ");
    collapse_spaces(&value)
}

/// Removes the channel suffixes YouTube adds to auto-generated artists.
fn clean_artist(value: &str) -> String {
    let value = clean_label(value);
    let lower = value.to_ascii_lowercase();
    let trimmed = lower
        .strip_suffix(" - topic")
        .or_else(|| lower.strip_suffix("vevo"))
        .map(|prefix| value[..prefix.len()].to_owned())
        .unwrap_or(value);
    collapse_spaces(&trimmed)
}

fn strip_noise_segments(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut result = String::new();
    let mut index = 0;
    while index < chars.len() {
        let close = match chars[index] {
            '(' => Some(')'),
            '[' => Some(']'),
            '{' => Some('}'),
            _ => None,
        };
        if let Some(close) = close {
            if let Some(offset) = chars[index + 1..].iter().position(|value| *value == close) {
                let content: String = chars[index + 1..index + 1 + offset].iter().collect();
                if !contains_noise(&content) {
                    result.push(' ');
                    result.push(chars[index]);
                    result.push_str(&content);
                    result.push(close);
                    result.push(' ');
                }
                index += offset + 2;
                continue;
            }
        }
        result.push(chars[index]);
        index += 1;
    }
    result
}

fn contains_noise(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    if NOISE_PHRASES.iter().any(|phrase| lower.contains(phrase)) {
        return true;
    }
    lower
        .split(|character: char| !character.is_alphanumeric())
        .filter(|token| !token.is_empty())
        .any(|token| NOISE_TOKENS.contains(&token))
}

fn collapse_spaces(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn split_dash(title: &str) -> Option<(&str, &str)> {
    let (left, right) = title.split_once(" - ")?;
    let (left, right) = (left.trim(), right.trim());
    (!left.is_empty()
        && !right.is_empty()
        && left.chars().count() <= 60
        && left.split_whitespace().count() <= 6)
        .then_some((left, right))
}

fn normalize_label(value: &str) -> String {
    value
        .chars()
        .filter(|character| character.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

fn check_cancelled(cancel: Option<&CancelFlag>) -> Result<(), LyricsError> {
    match cancel {
        Some(flag) if flag.is_cancelled() => Err(LyricsError::Cancelled),
        _ => Ok(()),
    }
}

/// Converts a record into word-timed lines. Returns `None` when the record has
/// no timing information (plain lyrics are not enough to align notes).
pub fn transcript_from_record(
    record: &LyricsRecord,
    language_hint: Option<&str>,
) -> Option<Transcript> {
    if !record.is_usable() {
        return None;
    }
    if let Some(lyrics_file) = record.lyrics_file.as_deref() {
        if let Some(transcript) = transcript_from_lyrics_file(lyrics_file, language_hint) {
            return Some(transcript);
        }
    }
    record
        .synced_lyrics
        .as_deref()
        .and_then(|lrc| transcript_from_lrc(lrc, language_hint))
}

fn best_record(records: Vec<ApiRecord>, duration_s: Option<u64>) -> Option<LyricsRecord> {
    records
        .into_iter()
        .filter(|record| !record.instrumental)
        .map(LyricsRecord::from)
        .filter(LyricsRecord::is_usable)
        .min_by(|left, right| score(left, duration_s).total_cmp(&score(right, duration_s)))
}

fn score(record: &LyricsRecord, duration_s: Option<u64>) -> f64 {
    let mut score = 0.0;
    if let Some(target) = duration_s {
        score += (record.duration_s - target as f64).abs().min(60.0);
    }
    if record.lyrics_file.is_none() {
        score += 4.0;
    }
    if !record.has_word_sync {
        score += 2.0;
    }
    if record.synced_lyrics.is_none() {
        score += 8.0;
    }
    score
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ApiRecord {
    #[serde(default)]
    track_name: String,
    #[serde(default)]
    artist_name: String,
    #[serde(default)]
    duration: f64,
    #[serde(default)]
    instrumental: bool,
    #[serde(default)]
    has_word_sync: bool,
    #[serde(default)]
    synced_lyrics: Option<String>,
    #[serde(default)]
    plain_lyrics: Option<String>,
    #[serde(default)]
    lyrics_file: Option<String>,
}

impl From<ApiRecord> for LyricsRecord {
    fn from(record: ApiRecord) -> Self {
        Self {
            track_name: record.track_name,
            artist_name: record.artist_name,
            duration_s: record.duration,
            has_word_sync: record.has_word_sync,
            synced_lyrics: record.synced_lyrics,
            plain_lyrics: record.plain_lyrics,
            lyrics_file: record.lyrics_file,
        }
    }
}

// -- Lyricsfile YAML ---------------------------------------------------------

#[derive(Debug, Default, Deserialize)]
struct LyricsFile {
    #[serde(default)]
    metadata: LyricsFileMetadata,
    #[serde(default)]
    lines: Vec<LyricsFileLine>,
}

#[derive(Debug, Default, Deserialize)]
struct LyricsFileMetadata {
    #[serde(default)]
    language: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
struct LyricsFileLine {
    #[serde(default)]
    text: String,
    #[serde(default)]
    start_ms: Option<u64>,
    #[serde(default)]
    end_ms: Option<u64>,
    #[serde(default)]
    words: Vec<LyricsFileWord>,
}

#[derive(Debug, Default, Deserialize)]
struct LyricsFileWord {
    #[serde(default)]
    text: String,
    #[serde(default)]
    start_ms: Option<u64>,
    #[serde(default)]
    end_ms: Option<u64>,
}

fn transcript_from_lyrics_file(yaml: &str, language_hint: Option<&str>) -> Option<Transcript> {
    let file: LyricsFile = serde_yaml_ng::from_str(yaml).ok()?;
    let language = decide_language(
        &lyrics_text(&file.lines),
        file.metadata.language.as_deref(),
        language_hint,
    );

    let starts: Vec<Option<f64>> = file
        .lines
        .iter()
        .map(|line| {
            line.start_ms.map(ms_to_s).or_else(|| {
                line.words
                    .iter()
                    .find_map(|word| word.start_ms)
                    .map(ms_to_s)
            })
        })
        .collect();

    let mut lines = Vec::new();
    let mut previous_start = 0.0_f64;

    for (index, line) in file.lines.iter().enumerate() {
        let Some(line_start) = starts[index] else {
            continue;
        };
        let next_start = starts[index + 1..].iter().flatten().next().copied();
        let line_start = line_start.max(previous_start);
        let line_end = line
            .end_ms
            .map(ms_to_s)
            .or(next_start)
            .unwrap_or(line_start + 4.0)
            .max(line_start + 0.2);

        let mut words = Vec::new();
        let word_timed = line.words.iter().any(|word| word.start_ms.is_some());
        if word_timed {
            for (word_index, word) in line.words.iter().enumerate() {
                let tokens = tokens(&word.text);
                if tokens.is_empty() {
                    continue;
                }
                let entry_start = word.start_ms.map(ms_to_s).unwrap_or(line_start);
                let entry_end = word
                    .end_ms
                    .map(ms_to_s)
                    .or_else(|| {
                        line.words[word_index + 1..]
                            .iter()
                            .find_map(|next| next.start_ms)
                            .map(ms_to_s)
                    })
                    .unwrap_or(line_end);
                push_tokens(
                    &mut words,
                    &tokens,
                    entry_start,
                    entry_end,
                    WORD_SYNC_CONFIDENCE,
                    &mut previous_start,
                );
            }
        } else {
            let tokens = tokens(&line.text);
            push_tokens(
                &mut words,
                &tokens,
                line_start,
                line_end,
                LINE_SYNC_CONFIDENCE,
                &mut previous_start,
            );
        }

        if !words.is_empty() {
            lines.push(TranscriptLine::new(words));
        }
    }

    transcript_from_lines(lines, &language)
}

// -- LRC fallback ------------------------------------------------------------

fn transcript_from_lrc(lrc: &str, language_hint: Option<&str>) -> Option<Transcript> {
    let mut entries: Vec<(f64, String)> = Vec::new();
    for raw in lrc.lines() {
        let mut rest = raw.trim_start();
        let mut timestamps = Vec::new();
        while let Some(stripped) = rest.strip_prefix('[') {
            let Some(end) = stripped.find(']') else {
                break;
            };
            let stamp = &stripped[..end];
            rest = &stripped[end + 1..];
            match parse_timestamp(stamp) {
                Some(seconds) => timestamps.push(seconds),
                None => break,
            }
        }
        let text = rest.trim();
        if timestamps.is_empty() || text.is_empty() {
            continue;
        }
        for seconds in timestamps {
            entries.push((seconds, text.to_owned()));
        }
    }
    entries.sort_by(|left, right| left.0.total_cmp(&right.0));
    entries.dedup_by(|left, right| (left.0 - right.0).abs() < 0.001 && left.1 == right.1);

    let text = entries
        .iter()
        .map(|(_, text)| text.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    let language = decide_language(&text, None, language_hint);
    let mut lines = Vec::new();
    let mut previous_start = 0.0_f64;

    for (index, (start, text)) in entries.iter().enumerate() {
        let end = entries
            .get(index + 1)
            .map(|(next, _)| *next)
            .unwrap_or(start + 5.0);
        let tokens = tokens(text);
        let mut words = Vec::new();
        push_tokens(
            &mut words,
            &tokens,
            *start,
            end,
            LINE_SYNC_CONFIDENCE,
            &mut previous_start,
        );
        if !words.is_empty() {
            lines.push(TranscriptLine::new(words));
        }
    }

    transcript_from_lines(lines, &language)
}

fn transcript_from_lines(lines: Vec<TranscriptLine>, language: &str) -> Option<Transcript> {
    if lines.is_empty() {
        return None;
    }
    Some(Transcript {
        language: language.to_owned(),
        language_confidence: 0.9,
        lines,
        discarded_segments: 0,
    })
}

fn push_tokens(
    output: &mut Vec<TranscriptWord>,
    tokens: &[String],
    start_s: f64,
    end_s: f64,
    confidence: f32,
    previous_start: &mut f64,
) {
    if tokens.is_empty() {
        return;
    }
    let start = start_s.max(*previous_start);
    let span = (end_s - start).max(0.1);
    let total: f64 = tokens
        .iter()
        .map(|token| token.chars().count().max(1) as f64)
        .sum();
    let mut consumed = 0.0;
    for token in tokens {
        let weight = token.chars().count().max(1) as f64;
        let token_start = start + span * (consumed / total);
        let token_end = start + span * ((consumed + weight) / total);
        consumed += weight;
        *previous_start = token_start;
        output.push(TranscriptWord {
            text: token.clone(),
            start_s: token_start,
            end_s: token_end.max(token_start + 0.05),
            confidence,
            ..TranscriptWord::default()
        });
    }
}

fn tokens(text: &str) -> Vec<String> {
    text.split_whitespace()
        .filter(|token| token.chars().any(char::is_alphanumeric))
        .map(str::to_owned)
        .collect()
}

fn parse_timestamp(value: &str) -> Option<f64> {
    let (minutes, rest) = value.trim().split_once(':')?;
    let minutes: f64 = minutes.trim().parse().ok()?;
    let seconds = if let Some((whole, fraction)) = rest.split_once('.') {
        let whole: f64 = whole.trim().parse().ok()?;
        let fraction: f64 = format!("0.{fraction}").parse().ok()?;
        whole + fraction
    } else if let Some((whole, hundredths)) = rest.split_once(':') {
        let whole: f64 = whole.trim().parse().ok()?;
        let hundredths: f64 = hundredths.trim().parse().ok()?;
        whole + hundredths / 100.0
    } else {
        rest.trim().parse().ok()?
    };
    Some(minutes * 60.0 + seconds)
}

fn ms_to_s(value: u64) -> f64 {
    value as f64 / 1000.0
}

fn normalize_language(value: &str) -> Option<String> {
    let base = value.trim().to_ascii_lowercase();
    if base.is_empty() {
        return None;
    }
    let code = base.split(['-', '_']).next().unwrap_or("").to_owned();
    let normalized = match code.as_str() {
        "english" => "en",
        "spanish" | "español" | "espanol" => "es",
        "french" | "français" | "francais" => "fr",
        "german" | "deutsch" => "de",
        "italian" | "italiano" => "it",
        "portuguese" | "português" | "portugues" => "pt",
        "japanese" | "日本語" => "ja",
        "korean" => "ko",
        "chinese" => "zh",
        "russian" => "ru",
        other => other,
    };
    (!normalized.is_empty()).then(|| normalized.to_owned())
}

fn default_language() -> String {
    "en".to_owned()
}

fn lyrics_text(lines: &[LyricsFileLine]) -> String {
    lines
        .iter()
        .map(|line| {
            if line.text.trim().is_empty() {
                line.words
                    .iter()
                    .map(|word| word.text.trim())
                    .collect::<Vec<_>>()
                    .join(" ")
            } else {
                line.text.clone()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Words that identify Spanish, French and English without depending on the
/// (often wrong) language tag in the Lyricsfile.
const SPANISH_MARKERS: &[&str] = &[
    "el", "la", "los", "las", "de", "del", "que", "y", "en", "un", "una", "por", "para", "con",
    "mi", "se", "te", "me", "lo", "le", "al", "es", "son", "soy", "voy", "vas", "vida", "amor",
    "corazon", "corazón", "bailar", "quiero", "quieres", "tengo", "tiene", "pero", "sin", "sobre",
    "cuando", "donde", "todo", "todos", "nada", "yo", "él", "ella", "nosotros", "ustedes", "está",
    "estás", "estoy", "eres", "fue", "ser", "estar", "hay", "dime", "noche", "cielo", "sueño",
    "sueños", "más",
];
const FRENCH_MARKERS: &[&str] = &[
    "les", "des", "une", "dans", "pour", "avec", "vous", "nous", "pas", "mon", "ton", "cette",
    "aux", "du", "au", "je", "qui", "mais", "comme", "sur", "être", "très", "aussi", "et", "est",
    "que",
];
const ENGLISH_MARKERS: &[&str] = &[
    "the", "and", "you", "your", "with", "that", "this", "have", "from", "like", "just", "can't",
    "won't", "i'm", "it's", "my", "me", "we", "they", "are", "was", "were", "what", "when",
    "where", "how", "why", "all", "love", "night", "day", "she", "he", "his", "her", "our",
    "their", "there", "here", "not", "don't", "of", "to", "in",
];

fn decide_language(text: &str, declared: Option<&str>, hint: Option<&str>) -> String {
    if let Some(hint) = hint.and_then(normalize_language) {
        return hint;
    }
    if let Some(script) = script_language(text) {
        return script.to_owned();
    }
    let declared = declared.and_then(normalize_language);
    let detected = classify_latin(text);
    match (declared, detected) {
        (Some(declared), Some(detected)) => {
            // The Lyricsfile language tag defaults to English for many
            // Spanish songs; trust the text when both disagree.
            if matches!(declared.as_str(), "en" | "es") && detected != declared {
                detected.to_owned()
            } else {
                declared
            }
        }
        (Some(declared), None) => declared,
        (None, Some(detected)) => detected.to_owned(),
        (None, None) => default_language(),
    }
}

fn classify_latin(text: &str) -> Option<&'static str> {
    let mut spanish = 0_i64;
    let mut french = 0_i64;
    let mut english = 0_i64;

    for raw in text.split_whitespace() {
        let token: String = raw
            .chars()
            .filter(|character| character.is_alphabetic())
            .collect::<String>()
            .to_lowercase();
        if token.is_empty() {
            continue;
        }
        if SPANISH_MARKERS.contains(&token.as_str()) {
            spanish += 1;
        }
        if FRENCH_MARKERS.contains(&token.as_str()) {
            french += 1;
        }
        if ENGLISH_MARKERS.contains(&token.as_str()) {
            english += 1;
        }
    }

    // Characters that only Spanish uses among these three languages.
    let accented = text
        .chars()
        .filter(|character| matches!(character, 'ñ' | '¿' | '¡' | 'á' | 'é' | 'í' | 'ó' | 'ú'))
        .count() as i64;
    spanish += accented.min(20);

    let best = spanish.max(french).max(english);
    if best == 0 {
        return None;
    }
    // Require a clear margin so short or mixed lines do not flip the language.
    let margin = 3;
    if spanish == best && spanish >= french + margin && spanish >= english + margin {
        Some("es")
    } else if french == best && french >= spanish + margin && french >= english + margin {
        Some("fr")
    } else if english == best && english >= spanish + margin && english >= french + margin {
        Some("en")
    } else {
        None
    }
}

fn script_language(text: &str) -> Option<&'static str> {
    let mut cyrillic = false;
    let mut greek = false;
    let mut arabic = false;
    let mut hebrew = false;
    let mut hangul = false;
    let mut kana = false;
    let mut han = false;
    for character in text.chars() {
        match character as u32 {
            0x0400..=0x04FF => cyrillic = true,
            0x0370..=0x03FF => greek = true,
            0x0600..=0x06FF => arabic = true,
            0x0590..=0x05FF => hebrew = true,
            0xAC00..=0xD7AF => hangul = true,
            0x3040..=0x30FF => kana = true,
            0x4E00..=0x9FFF => han = true,
            _ => {}
        }
    }
    if kana {
        Some("ja")
    } else if hangul {
        Some("ko")
    } else if han {
        Some("zh")
    } else if cyrillic {
        Some("ru")
    } else if greek {
        Some("el")
    } else if arabic {
        Some("ar")
    } else if hebrew {
        Some("he")
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const YAML: &str = r#"version: '1.0'
metadata:
  title: Yellow
  artist: Coldplay
  duration_ms: 267000
  instrumental: false
  language: en
lines:
- text: Look at the stars
  start_ms: 33370
  end_ms: 34667
  words:
  - text: 'Look '
    start_ms: 33370
  - text: 'at '
    start_ms: 33803
  - text: 'the '
    start_ms: 33944
  - text: stars
    start_ms: 34152
- text: Look how they shine for you
  start_ms: 36160
  end_ms: 38588
  words:
  - text: 'Look '
    start_ms: 36160
  - text: 'how '
    start_ms: 36351
  - text: 'they '
    start_ms: 36459
  - text: 'shine '
    start_ms: 36567
  - text: 'for '
    start_ms: 36850
  - text: you
    start_ms: 37282
"#;

    #[test]
    fn word_synced_lyricsfile_keeps_every_word_and_timing() {
        let transcript = transcript_from_lyrics_file(YAML, None).expect("lyrics");
        assert_eq!(transcript.language, "en");
        assert_eq!(transcript.lines.len(), 2);
        assert_eq!(
            transcript.text(),
            "Look at the stars\nLook how they shine for you"
        );

        let first = &transcript.lines[0].words;
        assert_eq!(first.len(), 4);
        assert!((first[0].start_s - 33.37).abs() < 1e-6);
        assert!((first[1].start_s - 33.803).abs() < 1e-6);
        assert!((first[3].end_s - 34.667).abs() < 1e-6);
        assert!(first.iter().all(|word| word.confidence > 0.8));
    }

    #[test]
    fn line_synced_lyrics_spread_words_across_the_line() {
        let lrc = "[00:10.00] Hola mundo\n[00:14.00] Otra línea\n[ar:Coldplay]\n";
        let transcript = transcript_from_lrc(lrc, Some("es")).expect("lyrics");
        assert_eq!(transcript.language, "es");
        assert_eq!(transcript.lines.len(), 2);
        assert_eq!(transcript.lines[0].words.len(), 2);
        assert!((transcript.lines[0].start_s - 10.0).abs() < 1e-6);
        let second = &transcript.lines[1].words[0];
        assert!((second.start_s - 14.0).abs() < 1e-6);
        assert!(transcript.words().all(|word| word.end_s > word.start_s));
    }

    #[test]
    fn repeated_lrc_timestamps_create_repeated_lines() {
        let lrc = "[00:01.00][00:20.00] coro\n";
        let transcript = transcript_from_lrc(lrc, None).expect("lyrics");
        assert_eq!(transcript.lines.len(), 2);
        assert_eq!(transcript.text(), "coro\ncoro");
    }

    #[test]
    fn records_without_timing_are_rejected() {
        let record = LyricsRecord {
            track_name: "Song".into(),
            artist_name: "Artist".into(),
            duration_s: 200.0,
            has_word_sync: false,
            synced_lyrics: None,
            plain_lyrics: Some("just words".into()),
            lyrics_file: None,
        };
        assert!(transcript_from_record(&record, None).is_none());
    }

    #[test]
    fn broken_yaml_falls_back_to_the_lrc_field() {
        let record = LyricsRecord {
            track_name: "Song".into(),
            artist_name: "Artist".into(),
            duration_s: 200.0,
            has_word_sync: false,
            synced_lyrics: Some("[00:05.00] hola".into()),
            plain_lyrics: Some("hola".into()),
            lyrics_file: Some("not: [valid".into()),
        };
        let transcript = transcript_from_record(&record, Some("es")).expect("lrc fallback");
        assert_eq!(transcript.text(), "hola");
    }

    #[test]
    fn records_are_ranked_by_duration_and_timing_quality() {
        let plain = ApiRecord {
            track_name: "Song".into(),
            artist_name: "Artist".into(),
            duration: 200.0,
            instrumental: false,
            has_word_sync: false,
            synced_lyrics: Some("[00:01.00] hola".into()),
            plain_lyrics: None,
            lyrics_file: None,
        };
        let synced = ApiRecord {
            duration: 201.0,
            has_word_sync: true,
            lyrics_file: Some(YAML.into()),
            synced_lyrics: Some("[00:01.00] hola".into()),
            ..plain.clone()
        };
        let far = ApiRecord {
            duration: 260.0,
            ..synced.clone()
        };
        let chosen = best_record(vec![plain, far, synced], Some(200)).expect("record");
        assert!(chosen.has_word_sync);
        assert!((chosen.duration_s - 201.0).abs() < 1e-6);
    }

    #[test]
    fn queries_are_built_from_noisy_youtube_metadata() {
        let query = LyricsQuery::from_source(
            "Walls - Mi nena (Videoclip Oficial)",
            Some("Walls - Topic"),
            Some(175_000),
        );
        assert_eq!(query.track_name, "Mi nena");
        assert_eq!(query.artist_name.as_deref(), Some("Walls"));
        assert_eq!(query.duration_s, Some(175));

        let query = LyricsQuery::from_source(
            "Rick Astley - Never Gonna Give You Up (Official Video)",
            None,
            Some(213_000),
        );
        assert_eq!(query.track_name, "Never Gonna Give You Up");
        assert_eq!(query.artist_name.as_deref(), Some("Rick Astley"));

        let query = LyricsQuery::from_source(
            "Earth, Wind & Fire - September",
            Some("Earth, Wind & Fire"),
            None,
        );
        assert_eq!(query.track_name, "September");
        assert_eq!(query.artist_name.as_deref(), Some("Earth, Wind & Fire"));
    }

    #[test]
    fn version_tags_that_change_the_recording_are_kept() {
        let query = LyricsQuery::from_source(
            "Coldplay - Yellow (Live at Glastonbury)",
            Some("ColdplayVEVO"),
            Some(267_000),
        );
        assert_eq!(query.track_name, "Yellow (Live at Glastonbury)");
        assert_eq!(query.artist_name.as_deref(), Some("Coldplay"));
    }

    #[test]
    fn language_codes_are_normalised() {
        assert_eq!(normalize_language("en-US").as_deref(), Some("en"));
        assert_eq!(normalize_language("Spanish").as_deref(), Some("es"));
        assert_eq!(normalize_language("  ").as_deref(), None);
    }

    #[test]
    fn spanish_lyrics_override_a_wrong_english_tag() {
        let text = "Voy a reír, voy a bailar\nVivir mi vida, la, la, la\nVoy a reír, voy a gozar";
        assert_eq!(decide_language(text, Some("en"), None), "es");
    }

    #[test]
    fn english_lyrics_stay_english() {
        let text = "Look at the stars\nLook how they shine for you\nAnd everything you do";
        assert_eq!(decide_language(text, Some("en"), None), "en");
        assert_eq!(decide_language(text, Some("es"), None), "en");
    }

    #[test]
    fn forced_language_wins_and_scripts_are_recognised() {
        assert_eq!(
            decide_language("Voy a bailar", Some("en"), Some("en")),
            "en"
        );
        assert_eq!(decide_language("Привет мир", None, None), "ru");
        assert_eq!(decide_language("こんにちは世界", None, None), "ja");
        assert_eq!(decide_language("مرحبا بالعالم", None, None), "ar");
    }

    #[test]
    fn timestamps_parse_in_the_supported_shapes() {
        assert!((parse_timestamp("01:02.50").unwrap() - 62.5).abs() < 1e-6);
        assert!((parse_timestamp("01:02:50").unwrap() - 62.5).abs() < 1e-6);
        assert!((parse_timestamp("00:07").unwrap() - 7.0).abs() < 1e-6);
        assert!(parse_timestamp("ar:Artist").is_none());
    }
}

//! Diagnostic runner: aligns a project's saved transcript and notes and prints
//! the first phrases with their real times.
//!
//! ```text
//! cargo run -p usdx-ml --example align_probe -- <project>/work
//! ```

use std::path::Path;

use usdx_domain::{SongHeader, Transcript};
use usdx_ml::{
    align_with_voiced, dictionary_syllabifier, load_vocals_16k, AlignmentOptions, EnergyTrack,
    QuantizedNote, QuantizedSong, WHISPER_SAMPLE_RATE,
};

fn main() {
    let work = std::env::args()
        .nth(1)
        .expect("usage: align_probe <project>/work");
    let work = Path::new(&work);
    let transcript: Transcript =
        serde_json::from_str(&std::fs::read_to_string(work.join("transcript.json")).unwrap())
            .unwrap();
    let artifacts: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(work.join("notes.json")).unwrap()).unwrap();

    let header_value = &artifacts["header"];
    let header = SongHeader {
        title: header_value["title"]
            .as_str()
            .unwrap_or_default()
            .to_string(),
        artist: header_value["artist"]
            .as_str()
            .unwrap_or_default()
            .to_string(),
        audio: "audio.mp3".into(),
        bpm: header_value["bpm"].as_f64().unwrap_or(0.0),
        gap_ms: header_value["gap_ms"].as_i64().unwrap_or(0),
        ..SongHeader::default()
    };
    let notes: Vec<QuantizedNote> = artifacts["notes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|note| QuantizedNote {
            start_beat: note["start_beat"].as_u64().unwrap() as u32,
            length_beats: note["length_beats"].as_u64().unwrap() as u32,
            pitch: note["pitch"].as_i64().unwrap() as i16,
            confidence: note["confidence"].as_f64().unwrap_or(0.0) as f32,
            adjusted: note["adjusted"].as_bool().unwrap_or(false),
        })
        .collect();
    let song = QuantizedSong {
        notes,
        adjustments: Vec::new(),
    };

    let voiced = load_vocals_16k(work.join("stems/vocals.wav"))
        .map(|samples| {
            let segments =
                EnergyTrack::analyze(&samples, WHISPER_SAMPLE_RATE).voiced_segments(1.2, 1.0);
            eprintln!("voiced segments: {segments:?}");
            segments
        })
        .unwrap_or_default();

    let alignment = align_with_voiced(
        &transcript,
        &song,
        &header,
        dictionary_syllabifier(),
        AlignmentOptions::default(),
        Some(&voiced),
    );
    eprintln!(
        "word_timed={} syllables={} assigned={} grouped={} synth={} review={} dropped={}",
        transcript.word_timed,
        alignment.stats.syllables,
        alignment.stats.assigned_syllables,
        alignment.stats.grouped_notes,
        alignment.stats.synthesised_notes,
        alignment.stats.review_notes,
        alignment.stats.dropped_notes
    );

    let ms = |beat: u32| header.gap_ms as f64 + beat as f64 * 60_000.0 / (header.bpm * 4.0);
    for (index, phrase) in alignment.phrases.iter().take(14).enumerate() {
        let text: String = phrase.notes.iter().map(|note| note.text.as_str()).collect();
        let start = phrase
            .notes
            .first()
            .map(|note| note.start_beat)
            .unwrap_or(0);
        let end = phrase
            .notes
            .last()
            .map(|note| note.start_beat + note.length_beats)
            .unwrap_or(start);
        println!(
            "P{index:02} [{:7.2}-{:7.2}s] review={} {}",
            ms(start) / 1000.0,
            ms(end) / 1000.0,
            phrase.needs_review,
            text
        );
    }
}

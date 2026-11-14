//! Manual check of the online lyrics lookup.
//!
//! ```bash
//! cargo run -p usdx-application --example lyrics_smoke -- "Coldplay" "Yellow" 267
//! ```

use usdx_application::{transcript_from_record, LyricsClient, LyricsQuery};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let artist = args.next().ok_or("artist is required")?;
    let title = args.next().ok_or("title is required")?;
    let duration = args.next().and_then(|value| value.parse::<u64>().ok());

    let query = LyricsQuery {
        track_name: title,
        artist_name: Some(artist),
        duration_s: duration,
    };
    let record = LyricsClient::new().lookup(&query, None).await?;
    let Some(record) = record else {
        println!("no timed lyrics found");
        return Ok(());
    };

    println!(
        "{} - {} · {:.0}s · word sync: {}",
        record.artist_name, record.track_name, record.duration_s, record.has_word_sync
    );
    match transcript_from_record(&record, None) {
        Some(transcript) => {
            println!(
                "{} lines, {} words, language {}",
                transcript.lines.len(),
                transcript.word_count(),
                transcript.language
            );
            for line in transcript.lines.iter().take(3) {
                println!("  [{:>7.2}] {}", line.start_s, line.text());
                if record.has_word_sync {
                    let words: Vec<String> = line
                        .words
                        .iter()
                        .map(|word| format!("{:.2}", word.start_s))
                        .collect();
                    println!("           {}", words.join(" "));
                }
            }
        }
        None => println!("the record has no usable timings"),
    }
    Ok(())
}

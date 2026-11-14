use usdx_application::{ModelCatalog, ModelDownloader, ModelStore};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let id = std::env::args().nth(1).unwrap_or_else(|| "htdemucs".into());
    let root = std::env::args()
        .nth(2)
        .map(std::path::PathBuf::from)
        .unwrap_or_else(ModelStore::default_root);

    let catalog = ModelCatalog::embedded()?;
    let spec = catalog
        .get(&id)
        .ok_or_else(|| format!("unknown model: {id}"))?
        .clone();
    let store = ModelStore::new(root);
    let downloader = ModelDownloader::new(store);
    let path = downloader
        .download(
            &spec,
            |downloaded, total| {
                if total > 0 {
                    eprintln!("{:.1}%", downloaded as f64 / total as f64 * 100.0);
                }
            },
            None,
        )
        .await?;
    println!("{}", path.display());
    Ok(())
}

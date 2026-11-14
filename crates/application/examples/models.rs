use usdx_application::{Hardware, ModelCatalog, ModelStore};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let catalog = ModelCatalog::embedded()?;
    let store = ModelStore::new(ModelStore::default_root());
    println!("models directory: {}", store.root().display());

    let mut total = 0_u64;
    for model in &catalog.models {
        total += model.size_bytes;
        println!(
            "{} | {:?} | {} MB | present={} | license={}",
            model.id,
            model.purpose,
            model.size_bytes / (1024 * 1024),
            store.is_present(model),
            model.license
        );
    }
    println!("total download size: {} MB", total / (1024 * 1024));

    let hardware = Hardware::detect();
    println!(
        "hardware: cuda={} coreml={}",
        hardware.cuda, hardware.coreml
    );
    Ok(())
}

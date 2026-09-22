//! Developer-only regeneration; production exports continue to protect existing files.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../contracts");
    std::fs::create_dir_all(&dir)?;
    for (name, document) in josh_app::contracts::documents() {
        std::fs::write(
            dir.join(name),
            format!("{}\n", serde_json::to_string_pretty(&document)?),
        )?;
    }
    Ok(())
}

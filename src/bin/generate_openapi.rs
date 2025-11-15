use api::api::ApiDoc;
use std::fs;
use std::path::Path;
use utoipa::OpenApi;

fn main() -> anyhow::Result<()> {
    // Generate OpenAPI spec
    let openapi = ApiDoc::openapi();

    // Serialize to JSON
    let json = openapi.to_pretty_json()?;

    // Write to docs/static/openapi.json
    let output_path = "./docs/static/openapi.json";

    // Create parent directory if it doesn't exist
    if let Some(parent) = Path::new(output_path).parent() {
        fs::create_dir_all(parent)?;
    }

    fs::write(output_path, json)?;

    println!("OpenAPI spec generated at: {}", output_path);

    Ok(())
}

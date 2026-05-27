mod types;
mod store;
mod engine;
mod server;

use rmcp::{ServiceExt, transport::stdio};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let manifest = adk_mcp_sdk::ServerManifest::from_file(std::path::Path::new("mcp-server.toml"))?;
    let errors = manifest.validate();
    if !errors.is_empty() { for e in &errors { eprintln!("  - {e}"); } }
    let server = server::PricingServer::new();
    let service = server.serve(stdio()).await?;
    service.waiting().await?;
    Ok(())
}

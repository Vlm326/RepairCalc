use std::{env, error::Error, path::PathBuf, time::Duration};

use repair_calc::catalog::{CatalogClient, ClientConfig, SyncOptions, synchronize_to_file};

fn env_value<T: std::str::FromStr>(name: &str, default: T) -> Result<T, Box<dyn Error>> {
    match env::var(name) {
        Ok(value) => value.parse().map_err(|_| format!("invalid {name}").into()),
        Err(env::VarError::NotPresent) => Ok(default),
        Err(error) => Err(error.into()),
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let base_url = env::var("MAXIDOM_BASE_URL")
        .map_err(|_| "set MAXIDOM_BASE_URL to the catalog stand origin (without /api)")?;
    let mut config = ClientConfig::new(base_url);
    config.locale = env::var("MAXIDOM_LOCALE").unwrap_or_else(|_| "ru".into());
    config.bearer_token = env::var("MAXIDOM_API_TOKEN").ok();
    config.timeout = Duration::from_secs(env_value("MAXIDOM_TIMEOUT_SECONDS", 30)?);
    let options = SyncOptions {
        root_category_id: env_value("MAXIDOM_ROOT_CATEGORY_ID", 1)?,
        depth_level: env_value("MAXIDOM_DEPTH_LEVEL", 20)?,
        per_page: env_value("MAXIDOM_PER_PAGE", 20)?,
    };
    let path = PathBuf::from(
        env::var("MAXIDOM_SNAPSHOT_PATH").unwrap_or_else(|_| "data/catalog.json".into()),
    );
    let client = CatalogClient::new(config)?;
    let snapshot = synchronize_to_file(&client, &options, path.clone()).await?;
    println!(
        "Catalog synchronized: {} categories, {} products, {} regions -> {}",
        snapshot.categories.len(),
        snapshot.products.len(),
        snapshot.regions.len(),
        path.display()
    );
    Ok(())
}

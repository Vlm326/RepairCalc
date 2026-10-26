use repair_calc::storage::CatalogStore;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let store = CatalogStore::connect(&std::env::var("DATABASE_URL")?).await?;
    store.migrate().await?;
    println!("Catalog migrations applied");
    Ok(())
}

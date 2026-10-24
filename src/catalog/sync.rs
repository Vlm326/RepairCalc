use std::{
    collections::{BTreeMap, BTreeSet},
    io::{BufWriter, Write},
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};

use super::{CatalogClient, Error, ProductQuery, merge_category_context};
use crate::models::{
    catalog::{Category, Product},
    region::Region,
};

#[derive(Debug, Clone)]
pub struct SyncOptions {
    pub root_category_id: u64,
    pub depth_level: u32,
    pub per_page: u64,
}

impl Default for SyncOptions {
    fn default() -> Self {
        Self {
            root_category_id: 1,
            depth_level: 20,
            per_page: 20,
        }
    }
}

/// Complete snapshot for the requested subtree. Products use Bagisto IDs as keys.
/// A new successful sync replaces this scope, including removals from the source.
#[derive(Debug, Serialize, Deserialize)]
pub struct CatalogSnapshot {
    pub synced_at_unix: u64,
    pub root_category_id: u64,
    pub depth_level: u32,
    pub categories_tree: Vec<Category>,
    pub categories: BTreeMap<u64, Category>,
    pub products: BTreeMap<u64, Product>,
    pub regions: BTreeMap<u64, Region>,
}

/// Fetch all pages of every category, including non-leaf categories and the root.
/// Nothing is published if any request fails. Calls are sequential to limit stand load.
pub async fn synchronize(
    client: &CatalogClient,
    options: &SyncOptions,
) -> Result<CatalogSnapshot, Error> {
    if options.depth_level == 0 || options.per_page == 0 {
        return Err(Error::Config(
            "sync depth_level and per_page must be positive".into(),
        ));
    }
    let mut tree = client
        .category_tree(options.root_category_id, options.depth_level)
        .await?
        .data;
    let mut categories = BTreeMap::new();
    let mut parents = BTreeSet::new();
    let mut pending: Vec<_> = tree.iter().collect();
    while let Some(category) = pending.pop() {
        if categories.contains_key(&category.id) {
            return Err(Error::Protocol(format!(
                "duplicate category {} in tree",
                category.id
            )));
        }
        if let Some(parent) = category.parent_id {
            parents.insert((parent, category.depth_level.unwrap_or(options.depth_level)));
        }
        let mut flat = category.clone();
        flat.children.clear();
        categories.insert(category.id, flat);
        pending.extend(&category.children);
    }
    // Category descriptions are only guaranteed by the separate category endpoint.
    for (parent, depth) in parents {
        for mut category in client.categories(parent, depth).await?.data {
            if let Some(existing) = categories.get_mut(&category.id) {
                category.children.clear();
                *existing = category;
            }
        }
    }
    fn enrich_tree(nodes: &mut [Category], categories: &BTreeMap<u64, Category>) {
        for node in nodes {
            if let Some(category) = categories.get(&node.id) {
                let children = std::mem::take(&mut node.children);
                *node = category.clone();
                node.children = children;
            }
            enrich_tree(&mut node.children, categories);
        }
    }
    enrich_tree(&mut tree, &categories);

    let regions = client
        .regions()
        .await?
        .data
        .into_iter()
        .map(|r| (r.id, r))
        .collect();
    let ids: BTreeSet<_> = categories
        .keys()
        .copied()
        .chain([options.root_category_id])
        .collect();
    let mut products = BTreeMap::new();
    for category_id in ids {
        for summary in client
            .products(ProductQuery::new(category_id, options.per_page))
            .await?
        {
            // A product may belong to several categories. Fetch its full card only once.
            if let Some(existing) = products.get_mut(&summary.id) {
                merge_category_context(existing, summary);
                continue;
            }
            let mut product = client.product(summary.id).await?;
            merge_category_context(&mut product, summary);
            products.insert(product.id, product);
        }
    }
    Ok(CatalogSnapshot {
        synced_at_unix: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| Error::Protocol("system clock predates Unix epoch".into()))?
            .as_secs(),
        root_category_id: options.root_category_id,
        depth_level: options.depth_level,
        categories_tree: tree,
        categories,
        products,
        regions,
    })
}

impl CatalogSnapshot {
    /// Write a temporary file in the destination directory, flush, then atomically rename.
    pub fn save(&self, path: impl AsRef<Path>) -> Result<(), Error> {
        let path = path.as_ref();
        let parent = path
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new("."));
        std::fs::create_dir_all(parent)?;
        let mut temporary = tempfile::NamedTempFile::new_in(parent)?;
        {
            let mut writer = BufWriter::new(temporary.as_file_mut());
            serde_json::to_writer(&mut writer, self)?;
            writer.flush()?;
        }
        temporary.as_file().sync_all()?;
        temporary
            .persist(path)
            .map_err(|error| Error::Io(error.error))?;
        Ok(())
    }

    pub fn load(path: impl AsRef<Path>) -> Result<Self, Error> {
        Ok(serde_json::from_reader(std::io::BufReader::new(
            std::fs::File::open(path)?,
        ))?)
    }
}

pub async fn synchronize_to_file(
    client: &CatalogClient,
    options: &SyncOptions,
    path: impl Into<PathBuf>,
) -> Result<CatalogSnapshot, Error> {
    let snapshot = synchronize(client, options).await?;
    let path = path.into();
    tokio::task::spawn_blocking(move || {
        snapshot.save(path)?;
        Ok::<_, Error>(snapshot)
    })
    .await?
}

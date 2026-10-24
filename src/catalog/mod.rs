//! Maxidom API client and complete, atomically published catalog snapshots.
mod client;
mod sync;

pub use client::{CatalogClient, ClientConfig, Error, ProductQuery};
pub use sync::{CatalogSnapshot, SyncOptions, synchronize, synchronize_to_file};

fn merge_category_context(
    product: &mut crate::models::catalog::Product,
    summary: crate::models::catalog::Product,
) {
    if product.categories_tree.is_none() {
        product.categories_tree = summary.categories_tree;
    }
    if let Some(categories) = summary.categories {
        let current = product.categories.get_or_insert_with(Default::default);
        for (id, name) in categories {
            current.entry(id).or_insert(name);
        }
    }
}

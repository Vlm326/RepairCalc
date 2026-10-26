//! PostgreSQL catalog. All monetary amounts use Decimal, never floating point.
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgPool, postgres::PgPoolOptions};

pub static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./migrations");

#[derive(Clone)]
pub struct CatalogStore {
    pool: PgPool,
}
impl CatalogStore {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
    pub async fn connect(url: &str) -> Result<Self, sqlx::Error> {
        Ok(Self::new(
            PgPoolOptions::new().max_connections(5).connect(url).await?,
        ))
    }
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }
    pub async fn migrate(&self) -> Result<(), sqlx::migrate::MigrateError> {
        MIGRATOR.run(&self.pool).await
    }
    pub async fn add_category(&self, product_id: i64, category_id: i64) -> Result<(), sqlx::Error> {
        sqlx::query("INSERT INTO catalog.product_categories (product_id,category_id) VALUES ($1,$2) ON CONFLICT DO NOTHING")
            .bind(product_id).bind(category_id).execute(&self.pool).await?;
        Ok(())
    }
    pub async fn remove_category(
        &self,
        product_id: i64,
        category_id: i64,
    ) -> Result<bool, sqlx::Error> {
        Ok(sqlx::query(
            "DELETE FROM catalog.product_categories WHERE product_id=$1 AND category_id=$2",
        )
        .bind(product_id)
        .bind(category_id)
        .execute(&self.pool)
        .await?
        .rows_affected()
            > 0)
    }
    pub async fn products_in_category(
        &self,
        category_id: i64,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<StoredProduct>, sqlx::Error> {
        sqlx::query_as("SELECT p.* FROM catalog.products p JOIN catalog.product_categories c ON c.product_id=p.id WHERE c.category_id=$1 ORDER BY p.id LIMIT $2 OFFSET $3")
            .bind(category_id).bind(limit.clamp(1,1000)).bind(offset.max(0)).fetch_all(&self.pool).await
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Category {
    pub id: i64,
    pub name: String,
    pub description: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategoryInput {
    pub name: String,
    pub description: String,
}
impl CatalogStore {
    pub async fn create_category(&self, input: &CategoryInput) -> Result<Category, sqlx::Error> {
        sqlx::query_as(
            "INSERT INTO catalog.categories (name, description) VALUES ($1, $2) RETURNING *",
        )
        .bind(&input.name)
        .bind(&input.description)
        .fetch_one(&self.pool)
        .await
    }
    pub async fn get_category(&self, id: i64) -> Result<Option<Category>, sqlx::Error> {
        sqlx::query_as("SELECT * FROM catalog.categories WHERE id=$1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await
    }
    pub async fn list_categories(
        &self,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Category>, sqlx::Error> {
        sqlx::query_as("SELECT * FROM catalog.categories ORDER BY id LIMIT $1 OFFSET $2")
            .bind(limit.clamp(1, 1000))
            .bind(offset.max(0))
            .fetch_all(&self.pool)
            .await
    }
    pub async fn update_category(
        &self,
        id: i64,
        input: &CategoryInput,
    ) -> Result<Option<Category>, sqlx::Error> {
        sqlx::query_as(
            "UPDATE catalog.categories SET name=$1, description=$2 WHERE id=$3 RETURNING *",
        )
        .bind(&input.name)
        .bind(&input.description)
        .bind(id)
        .fetch_optional(&self.pool)
        .await
    }
    pub async fn delete_category(&self, id: i64) -> Result<bool, sqlx::Error> {
        Ok(sqlx::query("DELETE FROM catalog.categories WHERE id=$1")
            .bind(id)
            .execute(&self.pool)
            .await?
            .rows_affected()
            > 0)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct StoredProduct {
    pub id: i64,
    pub sku: String,
    pub name: String,
    pub description: String,
    pub unit: String,
    pub package_quantity: Decimal,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductInput {
    pub sku: String,
    pub name: String,
    pub description: String,
    pub unit: String,
    pub package_quantity: Decimal,
}
impl CatalogStore {
    pub async fn create_product(&self, input: &ProductInput) -> Result<StoredProduct, sqlx::Error> {
        sqlx::query_as("INSERT INTO catalog.products (sku, name, description, unit, package_quantity) VALUES ($1, $2, $3, $4, $5) RETURNING *")
            .bind(&input.sku).bind(&input.name).bind(&input.description).bind(&input.unit).bind(input.package_quantity).fetch_one(&self.pool).await
    }
    pub async fn get_product(&self, id: i64) -> Result<Option<StoredProduct>, sqlx::Error> {
        sqlx::query_as("SELECT * FROM catalog.products WHERE id=$1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await
    }
    pub async fn list_products(
        &self,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<StoredProduct>, sqlx::Error> {
        sqlx::query_as("SELECT * FROM catalog.products ORDER BY id LIMIT $1 OFFSET $2")
            .bind(limit.clamp(1, 1000))
            .bind(offset.max(0))
            .fetch_all(&self.pool)
            .await
    }
    pub async fn update_product(
        &self,
        id: i64,
        input: &ProductInput,
    ) -> Result<Option<StoredProduct>, sqlx::Error> {
        sqlx::query_as("UPDATE catalog.products SET sku=$1, name=$2, description=$3, unit=$4, package_quantity=$5 WHERE id=$6 RETURNING *")
            .bind(&input.sku).bind(&input.name).bind(&input.description).bind(&input.unit).bind(input.package_quantity).bind(id).fetch_optional(&self.pool).await
    }
    pub async fn delete_product(&self, id: i64) -> Result<bool, sqlx::Error> {
        Ok(sqlx::query("DELETE FROM catalog.products WHERE id=$1")
            .bind(id)
            .execute(&self.pool)
            .await?
            .rows_affected()
            > 0)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Characteristic {
    pub id: i64,
    pub code: String,
    pub name: String,
    pub unit: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacteristicInput {
    pub code: String,
    pub name: String,
    pub unit: Option<String>,
}
impl CatalogStore {
    pub async fn create_characteristic(
        &self,
        input: &CharacteristicInput,
    ) -> Result<Characteristic, sqlx::Error> {
        sqlx::query_as("INSERT INTO catalog.characteristics (code, name, unit) VALUES ($1, $2, $3) RETURNING *")
            .bind(&input.code).bind(&input.name).bind(&input.unit).fetch_one(&self.pool).await
    }
    pub async fn get_characteristic(&self, id: i64) -> Result<Option<Characteristic>, sqlx::Error> {
        sqlx::query_as("SELECT * FROM catalog.characteristics WHERE id=$1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await
    }
    pub async fn list_characteristics(
        &self,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Characteristic>, sqlx::Error> {
        sqlx::query_as("SELECT * FROM catalog.characteristics ORDER BY id LIMIT $1 OFFSET $2")
            .bind(limit.clamp(1, 1000))
            .bind(offset.max(0))
            .fetch_all(&self.pool)
            .await
    }
    pub async fn update_characteristic(
        &self,
        id: i64,
        input: &CharacteristicInput,
    ) -> Result<Option<Characteristic>, sqlx::Error> {
        sqlx::query_as(
            "UPDATE catalog.characteristics SET code=$1, name=$2, unit=$3 WHERE id=$4 RETURNING *",
        )
        .bind(&input.code)
        .bind(&input.name)
        .bind(&input.unit)
        .bind(id)
        .fetch_optional(&self.pool)
        .await
    }
    pub async fn delete_characteristic(&self, id: i64) -> Result<bool, sqlx::Error> {
        Ok(
            sqlx::query("DELETE FROM catalog.characteristics WHERE id=$1")
                .bind(id)
                .execute(&self.pool)
                .await?
                .rows_affected()
                > 0,
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ProductCharacteristic {
    pub id: i64,
    pub product_id: i64,
    pub characteristic_id: i64,
    pub value: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductCharacteristicInput {
    pub product_id: i64,
    pub characteristic_id: i64,
    pub value: String,
}
impl CatalogStore {
    pub async fn create_product_characteristic(
        &self,
        input: &ProductCharacteristicInput,
    ) -> Result<ProductCharacteristic, sqlx::Error> {
        sqlx::query_as("INSERT INTO catalog.product_characteristics (product_id, characteristic_id, value) VALUES ($1, $2, $3) RETURNING *")
            .bind(input.product_id).bind(input.characteristic_id).bind(&input.value).fetch_one(&self.pool).await
    }
    pub async fn get_product_characteristic(
        &self,
        id: i64,
    ) -> Result<Option<ProductCharacteristic>, sqlx::Error> {
        sqlx::query_as("SELECT * FROM catalog.product_characteristics WHERE id=$1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await
    }
    pub async fn list_product_characteristics(
        &self,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<ProductCharacteristic>, sqlx::Error> {
        sqlx::query_as(
            "SELECT * FROM catalog.product_characteristics ORDER BY id LIMIT $1 OFFSET $2",
        )
        .bind(limit.clamp(1, 1000))
        .bind(offset.max(0))
        .fetch_all(&self.pool)
        .await
    }
    pub async fn update_product_characteristic(
        &self,
        id: i64,
        input: &ProductCharacteristicInput,
    ) -> Result<Option<ProductCharacteristic>, sqlx::Error> {
        sqlx::query_as("UPDATE catalog.product_characteristics SET product_id=$1, characteristic_id=$2, value=$3 WHERE id=$4 RETURNING *")
            .bind(input.product_id).bind(input.characteristic_id).bind(&input.value).bind(id).fetch_optional(&self.pool).await
    }
    pub async fn delete_product_characteristic(&self, id: i64) -> Result<bool, sqlx::Error> {
        Ok(
            sqlx::query("DELETE FROM catalog.product_characteristics WHERE id=$1")
                .bind(id)
                .execute(&self.pool)
                .await?
                .rows_affected()
                > 0,
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Price {
    pub id: i64,
    pub product_id: i64,
    pub region: String,
    pub currency: String,
    pub amount: Decimal,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceInput {
    pub product_id: i64,
    pub region: String,
    pub currency: String,
    pub amount: Decimal,
}
impl CatalogStore {
    pub async fn create_price(&self, input: &PriceInput) -> Result<Price, sqlx::Error> {
        sqlx::query_as("INSERT INTO catalog.prices (product_id, region, currency, amount) VALUES ($1, $2, $3, $4) RETURNING *")
            .bind(input.product_id).bind(&input.region).bind(&input.currency).bind(input.amount).fetch_one(&self.pool).await
    }
    pub async fn get_price(&self, id: i64) -> Result<Option<Price>, sqlx::Error> {
        sqlx::query_as("SELECT * FROM catalog.prices WHERE id=$1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await
    }
    pub async fn list_prices(&self, limit: i64, offset: i64) -> Result<Vec<Price>, sqlx::Error> {
        sqlx::query_as("SELECT * FROM catalog.prices ORDER BY id LIMIT $1 OFFSET $2")
            .bind(limit.clamp(1, 1000))
            .bind(offset.max(0))
            .fetch_all(&self.pool)
            .await
    }
    pub async fn update_price(
        &self,
        id: i64,
        input: &PriceInput,
    ) -> Result<Option<Price>, sqlx::Error> {
        sqlx::query_as("UPDATE catalog.prices SET product_id=$1, region=$2, currency=$3, amount=$4 WHERE id=$5 RETURNING *")
            .bind(input.product_id).bind(&input.region).bind(&input.currency).bind(input.amount).bind(id).fetch_optional(&self.pool).await
    }
    pub async fn delete_price(&self, id: i64) -> Result<bool, sqlx::Error> {
        Ok(sqlx::query("DELETE FROM catalog.prices WHERE id=$1")
            .bind(id)
            .execute(&self.pool)
            .await?
            .rows_affected()
            > 0)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Stock {
    pub id: i64,
    pub product_id: i64,
    pub region: String,
    pub warehouse: String,
    pub quantity: Decimal,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StockInput {
    pub product_id: i64,
    pub region: String,
    pub warehouse: String,
    pub quantity: Decimal,
}
impl CatalogStore {
    pub async fn create_stock(&self, input: &StockInput) -> Result<Stock, sqlx::Error> {
        sqlx::query_as("INSERT INTO catalog.stocks (product_id, region, warehouse, quantity) VALUES ($1, $2, $3, $4) RETURNING *")
            .bind(input.product_id).bind(&input.region).bind(&input.warehouse).bind(input.quantity).fetch_one(&self.pool).await
    }
    pub async fn get_stock(&self, id: i64) -> Result<Option<Stock>, sqlx::Error> {
        sqlx::query_as("SELECT * FROM catalog.stocks WHERE id=$1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await
    }
    pub async fn list_stocks(&self, limit: i64, offset: i64) -> Result<Vec<Stock>, sqlx::Error> {
        sqlx::query_as("SELECT * FROM catalog.stocks ORDER BY id LIMIT $1 OFFSET $2")
            .bind(limit.clamp(1, 1000))
            .bind(offset.max(0))
            .fetch_all(&self.pool)
            .await
    }
    pub async fn update_stock(
        &self,
        id: i64,
        input: &StockInput,
    ) -> Result<Option<Stock>, sqlx::Error> {
        sqlx::query_as("UPDATE catalog.stocks SET product_id=$1, region=$2, warehouse=$3, quantity=$4 WHERE id=$5 RETURNING *")
            .bind(input.product_id).bind(&input.region).bind(&input.warehouse).bind(input.quantity).bind(id).fetch_optional(&self.pool).await
    }
    pub async fn delete_stock(&self, id: i64) -> Result<bool, sqlx::Error> {
        Ok(sqlx::query("DELETE FROM catalog.stocks WHERE id=$1")
            .bind(id)
            .execute(&self.pool)
            .await?
            .rows_affected()
            > 0)
    }
}

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategoryTreeResponse {
    pub data: Vec<Category>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategoryListResponse {
    pub data: Vec<Category>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Category {
    pub id: u64,
    pub parent_id: Option<u64>,
    pub name: String,
    pub url_key: Option<String>,
    pub status: Option<bool>,
    pub depth_level: Option<u32>,
    pub product_count: Option<u64>,
    pub product_count_all_children: Option<u64>,
    #[serde(default)]
    pub children: Vec<Category>,
    pub description: Option<String>,
    pub is_hide: Option<String>,
    pub dat_show_b: Option<String>,
    pub dat_show_e: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductListResponse {
    pub items: Vec<Product>,
    #[serde(default)]
    pub properties: Vec<FilterProperty>,
    pub generated_url: Option<String>,
    pub meta: Option<Pagination>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductDetailsResponse {
    pub data: Product,
}

/// Сводная модель: краткий список и полная карточка имеют общий набор полей,
/// но подробные характеристики присутствуют только в полной карточке.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Product {
    pub id: u64,
    pub sku: Option<String>,
    pub cod_good: Option<String>,
    pub name: String,
    pub marketing_description: Option<String>,
    #[serde(rename = "type")]
    pub product_type: Option<String>,
    pub url_key: Option<String>,

    pub price: Option<f64>,
    pub result_price: Option<f64>,
    pub currency: Option<String>,
    pub old_price: Option<f64>,
    pub discount_amount: Option<f64>,
    pub discount_percent: Option<f64>,
    pub special_price: Option<f64>,
    pub special_price_from: Option<String>,
    pub special_price_to: Option<String>,
    pub special_price_active: Option<bool>,

    pub quantity_in_package: Option<f64>,
    pub package_area: Option<f64>,
    pub price_per_m2: Option<f64>,
    pub price_per_m2_unit: Option<String>,
    pub unit: Option<String>,

    pub labels: Option<Vec<ProductLabel>>,
    pub qty: Option<f64>,
    pub stocks: Option<Vec<Stock>>,
    pub in_stock: Option<bool>,
    pub status: Option<bool>,
    pub brand: Option<NamedEntity>,
    pub collection: Option<NamedEntity>,
    pub country: Option<NamedEntity>,
    pub categories: Option<HashMap<String, String>>,
    pub categories_tree: Option<Vec<CategoryPathItem>>,
    pub matrix_type: Option<i64>,
    pub matrix_type_name: Option<String>,
    pub is_purchasable_online: Option<bool>,
    pub pickup_text: Option<String>,
    pub ratings: Option<Ratings>,
    pub recsys_rating: Option<f64>,
    #[serde(rename = "propertiesBottom")]
    pub properties_bottom: Option<Vec<ProductProperty>>,
    pub created_at: Option<String>,

    pub description: Option<String>,
    pub weight: Option<f64>,
    pub meta_title: Option<String>,
    pub meta_description: Option<String>,
    pub updated_at: Option<String>,
    pub characteristics: Option<Vec<ProductCharacteristic>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductLabel {
    pub code: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Stock {
    pub warehouse_id: Option<u64>,
    pub warehouse_name: Option<String>,
    pub region_id: Option<u64>,
    pub qty: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NamedEntity {
    pub id: Option<u64>,
    pub name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategoryPathItem {
    pub id: u64,
    pub name: String,
    pub depth: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ratings {
    pub rating: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductProperty {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductCharacteristic {
    pub code: String,
    pub name: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterProperty {
    pub code: String,
    pub name: String,
    #[serde(rename = "type")]
    pub property_type: Option<String>,
    #[serde(default)]
    pub values: Vec<FilterValue>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterValue {
    pub value: String,
    pub label: Option<String>,
    pub count: Option<u64>,
    pub is_available: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pagination {
    pub total: Option<u64>,
    pub per_page: Option<u64>,
    pub current_page: Option<u64>,
    pub last_page: Option<u64>,
}

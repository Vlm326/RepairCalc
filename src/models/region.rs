use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegionListResponse {
    pub data: Vec<Region>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Region {
    pub id: u64,
    pub code: Option<String>,
    pub name: String,
    pub oracle_cod_city: Option<i64>,
    pub status: Option<bool>,
    pub is_default: Option<bool>,
    pub priority: Option<i64>,
    pub warehouses_count: Option<u64>,
}

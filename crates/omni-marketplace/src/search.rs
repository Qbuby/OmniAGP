use serde::{Deserialize, Serialize};

use crate::asset_pack::AssetType;
use crate::license::License;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchFilter {
    pub query: Option<String>,
    pub asset_type: Option<AssetType>,
    pub tags: Vec<String>,
    pub license: Option<License>,
    pub min_rating: Option<f32>,
    pub resolution: Option<String>,
    pub sort_by: SortBy,
    pub page: u32,
    pub per_page: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SortBy {
    Relevance,
    Downloads,
    Rating,
    Newest,
    Updated,
}

impl Default for SearchFilter {
    fn default() -> Self {
        Self {
            query: None,
            asset_type: None,
            tags: vec![],
            license: None,
            min_rating: None,
            resolution: None,
            sort_by: SortBy::Relevance,
            page: 0,
            per_page: 20,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult<T> {
    pub items: Vec<T>,
    pub total: u64,
    pub page: u32,
    pub per_page: u32,
    pub has_more: bool,
}

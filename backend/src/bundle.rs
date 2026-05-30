use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct BundledItem {
    pub item_id: i64,
    pub item: String,
    pub quantity: f64,
}

#[derive(Debug, Serialize)]
pub struct Bundle {
    pub id: i64,
    pub user: String,
    pub items: Vec<BundledItem>,
    pub created_at: String,
    pub bundled: bool,
    pub fulfilled_at: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct BundleListEntry {
    pub id: i64,
    pub user: String,
    pub items: Vec<BundledItem>,
    pub created_at: String,
    pub bundled: bool,
}

#[derive(Debug, Deserialize)]
pub struct NewBundledItem {
    pub item_id: i64,
    pub quantity: f64,
}

#[derive(Debug, Deserialize)]
pub struct NewBundle {
    pub user: String,
    pub bundled: Option<bool>,
    pub items: Vec<NewBundledItem>,
}

#[derive(Debug, Deserialize)]
pub struct UpdatedBundle {
    pub id: i64,
    pub user: String,
    pub bundled: bool,
    pub fulfilled_at: Option<String>,
    pub items: Vec<NewBundledItem>,
}

#[derive(Debug, Deserialize)]
pub(super) struct BundleQuery {
    pub(super) id: i64,
}

#[derive(Debug, sqlx::FromRow)]
pub(super) struct BundleRecord {
    pub(super) id: i64,
    pub(super) user: String,
    pub(super) created_at: String,
    pub(super) bundled: bool,
    pub(super) fulfilled_at: Option<String>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct BundleSelectionItem {
    pub user: String,
    pub item_id: i64,
    pub item: String,
    pub quantity: f64,
}

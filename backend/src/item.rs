use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, sqlx::Type)]
#[serde(rename_all = "snake_case")]
#[sqlx(type_name = "TEXT", rename_all = "snake_case")]
pub enum QuantityType {
    Count,
    Grams,
    Ounces,
    Pounds,
    Liters,
    Milliliters,
    Gallons,
}

impl QuantityType {
    pub(super) fn as_str(&self) -> &'static str {
        match self {
            Self::Count => "count",
            Self::Grams => "grams",
            Self::Ounces => "ounces",
            Self::Pounds => "pounds",
            Self::Liters => "liters",
            Self::Milliliters => "milliliters",
            Self::Gallons => "gallons",
        }
    }
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct InventoryItem {
    pub id: i64,
    pub item: String,
    pub quantity: f64,
    pub quantity_type: QuantityType,
}

#[derive(Debug, Deserialize)]
pub struct NewInventoryItem {
    pub item: String,
    pub quantity: f64,
    pub quantity_type: QuantityType,
}

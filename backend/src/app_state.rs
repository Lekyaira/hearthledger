use sqlx::SqlitePool;

#[derive(Clone)]
pub struct AppState {
    pub(super) pool: SqlitePool,
}

impl AppState {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

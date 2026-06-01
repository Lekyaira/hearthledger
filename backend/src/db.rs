use sqlx::{SqlitePool, sqlite::SqlitePoolOptions};

pub async fn create_pool(database_url: &str) -> Result<SqlitePool, sqlx::Error> {
    SqlitePoolOptions::new()
        .max_connections(1)
        .after_connect(|connection, _metadata| {
            Box::pin(async move {
                sqlx::query("PRAGMA foreign_keys = ON")
                    .execute(connection)
                    .await?;
                Ok(())
            })
        })
        .connect(database_url)
        .await
}

pub async fn migrate(pool: &SqlitePool) -> Result<(), sqlx::migrate::MigrateError> {
    sqlx::migrate!("./migrations").run(pool).await
}

#[cfg(test)]
pub mod tests {
    use super::*;

    pub async fn test_pool() -> SqlitePool {
        let pool = create_pool("sqlite::memory:").await.unwrap();
        migrate(&pool).await.unwrap();
        seed_test_data(&pool).await;
        pool
    }

    async fn seed_test_data(pool: &SqlitePool) {
        sqlx::query("DELETE FROM users")
            .execute(pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM sqlite_sequence WHERE name = 'users'")
            .execute(pool)
            .await
            .unwrap();

        sqlx::query(
            r#"
            INSERT INTO inventory (item, quantity, quantity_type) VALUES
                ('Canned tomatoes', 24, 'count'),
                ('Dried black beans', 18, 'count'),
                ('All-purpose flour', 10.5, 'pounds'),
                ('Paper towels', 10, 'count'),
                ('Laundry detergent', 6, 'count')
            "#,
        )
        .execute(pool)
        .await
        .unwrap();

        sqlx::query(
            r#"
            INSERT INTO users (name, role) VALUES
                ('Test Member', 'member'),
                ('Test Admin', 'admin')
            "#,
        )
        .execute(pool)
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn migrations_reset_data_and_seed_admin_user() {
        let pool = create_pool("sqlite::memory:").await.unwrap();
        migrate(&pool).await.unwrap();

        let inventory_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM inventory")
            .fetch_one(&pool)
            .await
            .unwrap();
        let users: Vec<(i64, String, String)> =
            sqlx::query_as("SELECT id, name, role FROM users ORDER BY id")
                .fetch_all(&pool)
                .await
                .unwrap();

        assert_eq!(inventory_count, 0);
        assert_eq!(users, vec![(1, "Admin".to_owned(), "admin".to_owned())]);
    }
}

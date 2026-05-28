use std::env;

use anyhow::Context;
use backend::{app, migrate};
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "backend=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let database_url =
        env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite://hearthledger.db".to_owned());

    let connect_options = database_url
        .parse::<SqliteConnectOptions>()
        .with_context(|| format!("invalid DATABASE_URL: {database_url}"))?
        .create_if_missing(true)
        .journal_mode(SqliteJournalMode::Wal);

    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(connect_options)
        .await
        .context("failed to connect to sqlite database")?;

    migrate(&pool).await.context("failed to run migrations")?;

    let bind_addr = env::var("BIND_ADDR").unwrap_or_else(|_| "127.0.0.1:3000".to_owned());
    let listener = tokio::net::TcpListener::bind(&bind_addr)
        .await
        .with_context(|| format!("failed to bind {bind_addr}"))?;

    println!("backend listening on http://{bind_addr}");
    axum::serve(listener, app(pool)).await?;

    Ok(())
}

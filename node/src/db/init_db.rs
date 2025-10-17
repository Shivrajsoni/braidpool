use anyhow::Result;
use sqlx::{sqlite::SqliteConnectOptions, Executor, SqlitePool};
use std::{env, fs, path::Path, str::FromStr};

pub async fn init_db() -> Result<SqlitePool> {
    let home_dir = env::var("HOME")?;
    let db_dir = Path::new(&home_dir).join(".braidpool");
    let db_path = db_dir.join("braidpool.db");
    let schema_path = std::env::current_dir()
        .unwrap()
        .join("node/src/db/schema.sql");

    fs::create_dir_all(&db_dir)?;

    let db_url = format!("sqlite://{}", db_path.to_string_lossy());
    let db_exists = db_path.exists();
    let sql_lite_connections = SqliteConnectOptions::from_str(&db_url)?
        .with_regexp()
        .foreign_keys(true)
        .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal);

    let conn = if db_exists {
        log::info!("[INFO] Database already exists at {:?}", db_path);
        let pool = SqlitePool::connect_with(sql_lite_connections).await?;
        pool
    } else {
        let _file = std::fs::File::create_new(db_path.clone());
        let schema_sql = fs::read_to_string(&schema_path.as_path())?;
        let pool = SqlitePool::connect_with(sql_lite_connections).await?;
        pool.execute(schema_sql.as_str()).await?;
        log::info!("[INFO] Schema initialized successfully at {:?}", db_path);
        pool
    };

    Ok(conn)
}

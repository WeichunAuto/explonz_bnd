use sea_orm::DatabaseConnection;

use crate::infrastructure::database;

pub mod api;
pub mod application;
pub use explonz_shared::common;
pub mod config;
pub use explonz_shared::entity;
pub mod commons;
pub mod error;
pub mod infrastructure;
pub mod middleware;
pub mod request;
pub mod response;
pub mod service;

/// initialize all settings for logger and database
pub async fn init_all_settings() -> anyhow::Result<DatabaseConnection> {
    let db_connection = database::init().await?;
    Ok(db_connection)
}

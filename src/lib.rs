use sea_orm::DatabaseConnection;

use crate::infrastructure::database;

pub mod api;
pub mod application;
pub mod common;
pub mod config;
pub mod entity;
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

use sea_orm::DatabaseConnection;

pub mod api;
pub mod application;
mod auth;
pub mod common;
pub mod config;
pub mod database;
pub mod entity;
pub mod error;
pub(crate) mod handlers;
pub mod logger;
pub mod middleware;
pub mod request;
pub mod response;

/// initialize all settings for logger and database
pub async fn init_all_settings() -> anyhow::Result<DatabaseConnection> {
    let db_connection = database::init().await?;
    Ok(db_connection)
}

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Deserialize)]
pub struct CreateSpotRequest {
    pub name: String,
    pub location: String,
    pub latitude: f64,
    pub longitude: f64,
    pub description: String,     // 默认 ""
    pub photo_urls: Vec<String>, // 默认 []
    pub attributes: Value,       // 默认 []

    pub phone: Option<String>,
    pub website: Option<String>,
    pub opening_hours: Vec<OpeningHourInput>, // 最多 7 条，每天一条
}

#[derive(Debug, Deserialize)]
pub struct OpeningHourInput {
    pub day_of_week: i16, // 0=Sun … 6=Sat
    pub is_closed: bool,
    pub is_open_24h: bool,
    pub open_time: Option<String>, // "HH:MM"，is_closed=false 且 is_open_24h=false 时必填
    pub close_time: Option<String>, // "HH:MM"
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ImageUploadResponse {
    pub id: String,
    pub url: String,
}

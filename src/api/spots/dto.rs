use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct CreateSpotRequest {
    pub name: String,
    pub location: String,
    pub latitude: f64,
    pub longitude: f64,
    pub description: String,
    #[serde(default)]
    pub photo_urls: Vec<String>,
    #[serde(default)]
    pub label_ids: Vec<String>,

    pub phone: Option<String>,
    pub website: Option<String>,
    pub opening_hours: Vec<OpeningHourInput>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OpeningHourInput {
    pub day_of_week: i16, // 0=Sun … 6=Sat
    pub is_closed: bool,
    pub is_open_24h: bool,
    pub open_time: Option<String>, // "HH:MM"，is_closed=false 且 is_open_24h=false 时必填
    pub close_time: Option<String>, // "HH:MM"
}

#[derive(Debug, Deserialize)]
pub struct UpdateSpotRequest {
    pub name: String,
    pub location: String,
    pub latitude: f64,
    pub longitude: f64,
    pub description: String,
    #[serde(default)]
    pub photo_urls: Vec<String>,
    #[serde(default)]
    pub label_ids: Vec<String>,
    pub phone: Option<String>,
    pub website: Option<String>,
    pub opening_hours: Vec<OpeningHourInput>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ImageUploadResponse {
    pub id: String,
    pub url: String,
}

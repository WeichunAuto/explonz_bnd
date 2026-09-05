use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct CreateLabelRequest {
    pub name: String,
    pub description: String,
    pub icon: String,
}

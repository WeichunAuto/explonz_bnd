use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct EmailConfig {
    sender: Option<String>,
    password: Option<String>,
    api_key: Option<String>,
}

impl EmailConfig {
    pub fn get_sender(&self) -> &str {
        self.sender.as_deref().unwrap_or("bobby.wang@163.com")
    }

    pub fn get_password(&self) -> &str {
        self.password.as_deref().unwrap_or("WW_33_cc163!")
    }

    pub fn get_api_key(&self) -> &str {
        self.api_key.as_deref().unwrap_or_default()
    }
}

use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct EmailConfig {
    sender: Option<String>,
    password: Option<String>,
    api_key: Option<String>,
    attempts: Option<String>,
    otp_code_validity_time: Option<String>,
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

    pub fn get_attempts(&self) -> &str {
        self.attempts.as_deref().unwrap_or("5") // 默认最多尝试5次
    }

    pub fn get_otp_code_validity_time(&self) -> &str {
        self.otp_code_validity_time.as_deref().unwrap_or("5") // 默认otp code 的有效时间为5分钟
    }
}

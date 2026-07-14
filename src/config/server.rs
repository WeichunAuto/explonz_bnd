use serde::Deserialize;

/// Server configuration for HTTP server settings.
///
/// Contains host and port configuration for the web server.
#[derive(Debug, Deserialize)]
pub struct ServerConfig {
    /// HTTP server hostname or IP address to bind to
    host: Option<String>,
    /// HTTP server port to listen on
    port: Option<u16>,
}

impl ServerConfig {
    /// Returns the server host address with fallback to localhost.
    ///
    /// Default: `127.0.0.1`
    pub fn get_host(&self) -> String {
        self.host.clone().unwrap_or("127.0.0.1".to_string())
    }

    /// Returns the server port number with fallback to development port.
    ///
    /// Default: `3000`
    pub fn get_port(&self) -> u16 {
        self.port.unwrap_or(3000)
    }
}

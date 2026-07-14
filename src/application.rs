use crate::config::AppConfig;
use crate::{database, logger};
use axum::extract::{DefaultBodyLimit, Request};
use axum::http::Response;
use axum::Router;
use bytesize::ByteSize;
use sea_orm::DatabaseConnection;
use std::fmt::{Display, Formatter};
use std::net::SocketAddr;
use std::time::Duration;
use tower_http::cors;
use tower_http::cors::{AllowMethods, CorsLayer};
use tower_http::normalize_path::NormalizePathLayer;
use tower_http::timeout::TimeoutLayer;
use tower_http::trace::{OnResponse, TraceLayer};
use tracing::Span;

/// Application state shared across all request handlers.
///
/// Contains database connection pool and other shared resources.
#[derive(Clone)]
pub struct AppState {
    pub db: DatabaseConnection,
}

/// Server instance responsible for starting and configuring the HTTP server.
struct Server {
    config: &'static AppConfig,
}

/// Custom tracing implementation for response latency logging.
#[derive(Debug, Clone)]
struct LatencyOnResponse;

struct Latency(Duration);

impl AppState {
    /// Creates a new application state with the given database connection.
    fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    /// Returns a reference to the database connection.
    pub fn db(&self) -> &DatabaseConnection {
        &self.db
    }
}

/// Starts the application server with the provided router.
///
/// # Process
/// 1. Initializes logging system
/// 2. Establishes database connection
/// 3. Creates application state
/// 4. Starts HTTP server with configured routes
///
/// # Arguments
/// * `router` - The application router containing all route definitions
///
/// # Returns
/// * `anyhow::Result<()>` - Result indicating server startup success or failure
pub async fn run(router: Router<AppState>) -> anyhow::Result<()> {
    // Initialize logging and tracing
    logger::init();
    tracing::info!("Starting the application server......");

    // Initialize database connection
    let db_connection = database::init().await?;

    // Create application state with database connection
    let app_state = AppState::new(db_connection);

    // Create server instance and start
    let server = Server::new(AppConfig::get());
    server.start(app_state, router).await
}

impl Server {
    /// Creates a new server instance with the given configuration.
    fn new(config: &'static AppConfig) -> Self {
        Self { config }
    }

    /// Starts the HTTP server and begins listening for requests.
    ///
    /// # Arguments
    /// * `state` - Application state to be shared with handlers
    /// * `router` - Router containing the route definitions
    async fn start(&self, state: AppState, router: Router<AppState>) -> anyhow::Result<()> {
        let server_config = self.config.server();
        tracing::info!("Server config: {:?}", server_config);

        let routes = self.create_routes(state, router);

        let addr = format!("{}:{}", server_config.get_host(), server_config.get_port());

        let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
        tracing::info!("The Application is listening on: {}", addr);
        axum::serve(
            listener,
            routes.into_make_service_with_connect_info::<SocketAddr>(),
        )
        .await?;

        Ok(())
    }

    /// Configures routes with tracing middleware and application state.
    fn create_routes(&self, state: AppState, router: Router<AppState>) -> Router {
        // request timeout, default 60s
        let timeout = TimeoutLayer::new(Duration::from_secs(60));

        let body_limit = DefaultBodyLimit::max(
            // body size limit 10MB
            ByteSize::mib(10).as_u64() as usize,
        );

        let cors = CorsLayer::new()
            .allow_origin(cors::Any)
            .allow_methods(AllowMethods::list(vec![
                http::Method::GET,
                http::Method::POST,
                http::Method::PUT,
                http::Method::DELETE,
                http::Method::PATCH,
                http::Method::OPTIONS,
            ]))
            .allow_headers(cors::Any)
            .allow_credentials(false)
            .max_age(Duration::from_secs(3600));

        let tracing = TraceLayer::new_for_http()
            .make_span_with(|request: &Request| {
                let method = request.method();
                let path = request.uri().path();
                let id = xid::new(); // Generate unique request ID

                tracing::info_span!("Api Request: ", id = %id, method = %method, path = %path)
            })
            .on_request(())
            .on_failure(())
            .on_response(LatencyOnResponse);

        //  remove trailing slashes from request paths.
        let normalize_path = NormalizePathLayer::trim_trailing_slash();

        Router::new()
            .merge(router)
            .layer(timeout)
            .layer(body_limit)
            .layer(tracing)
            .layer(cors)
            .layer(normalize_path)
            .with_state(state)
    }
}

impl<B> OnResponse<B> for LatencyOnResponse {
    fn on_response(self, response: &Response<B>, latency: Duration, _span: &Span) {
        tracing::info!(
            latency = %Latency(latency),
            status = %response.status().as_u16(),
            "finished processing request."
        )
    }
}

impl Display for Latency {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if self.0.as_millis() > 0 {
            write!(f, "{} ms", self.0.as_millis())
        } else {
            write!(f, "{} us", self.0.as_micros())
        }
    }
}

use crate::response::ApiResponse;
use axum::body::Body;
use axum::extract::rejection::{JsonRejection, PathRejection, QueryRejection};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum_valid::ValidRejection;

#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("Not Found")]
    NotFoundError,

    #[error("Method Not Allowed")]
    MethodNotAllowedError,

    #[error("Biz Error: {0}")]
    BizError(String),

    #[error("Database Error: {0}")]
    DatabaseError(#[from] sea_orm::DbErr),

    #[error("Internal Server Error: {0}")]
    InternalError(#[from] anyhow::Error),

    #[error("Query Params Error: {0}")]
    QueryError(#[from] QueryRejection),

    #[error("Path Params Error: {0}")]
    PathError(#[from] PathRejection),

    #[error("Json Body Error: {0}")]
    JsonError(#[from] JsonRejection),

    #[error("Validation Error: {0}")]
    ValidationError(String),

    #[error("JWT Error: {0}")]
    JWTError(#[from] jsonwebtoken::errors::Error),

    #[error("UnAuthorized Error: {0}")]
    UnAuthenticatedError(String),
}

impl ApiError {
    pub fn status_code(&self) -> StatusCode {
        match self {
            ApiError::NotFoundError => StatusCode::NOT_FOUND,
            ApiError::MethodNotAllowedError => StatusCode::METHOD_NOT_ALLOWED,
            ApiError::BizError(_) => StatusCode::OK,
            ApiError::DatabaseError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            ApiError::InternalError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            ApiError::QueryError(_)
            | ApiError::PathError(_)
            | ApiError::JsonError(_)
            | ApiError::ValidationError(_) => StatusCode::BAD_REQUEST,
            ApiError::JWTError(_) | ApiError::UnAuthenticatedError(_) => StatusCode::UNAUTHORIZED,
        }
    }
}
impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status_code = self.status_code();
        let body = axum::Json(ApiResponse::<()>::error(self.to_string()));
        (status_code, body).into_response()
    }
}

impl From<ValidRejection<ApiError>> for ApiError {
    fn from(value: ValidRejection<ApiError>) -> Self {
        match value {
            ValidRejection::Valid(errors) => ApiError::ValidationError(errors.to_string()),
            ValidRejection::Inner(errors) => errors,
        }
    }
}

impl From<ApiError> for Response<Body> {
    fn from(value: ApiError) -> Self {
        value.into_response()
    }
}

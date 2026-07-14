use crate::error::ApiError;
use axum::extract::{FromRequest, FromRequestParts, Request};
use axum_valid::HasValidate;
use http::request::Parts;
use regex::Regex;
use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::LazyLock;
use validator::ValidationError;

/// Custom Query extractor wrapper
///
/// Wraps Axum's built-in Query extractor with custom error handling.
/// Provides a consistent API error type for query parameter extraction failures.
#[derive(Debug, Clone, Copy, Default, FromRequestParts)]
#[from_request(via(axum::extract::Query), rejection(ApiError))]
pub struct BQuery<T>(pub T);

/// Custom Path extractor wrapper
#[derive(Debug, Clone, Copy, Default, FromRequestParts)]
#[from_request(via(axum::extract::Path), rejection(ApiError))]
pub struct BPath<T>(pub T);

/// Custom JSON extractor wrapper
#[derive(Debug, Clone, Copy, Default, FromRequest)]
#[from_request(via(axum::extract::Json), rejection(ApiError))]
pub struct BJson<T>(pub T);

/// Custom validation wrapper
#[derive(Debug, Clone, Copy, Default, FromRequestParts, FromRequest)]
#[from_request(via(axum_valid::Valid), rejection(ApiError))]
pub struct BValid<T>(pub T);

/// Validated Query extractor
///
/// Combines query parameter extraction with automatic validation.
/// Extracts and validates query parameters in a single operation.
#[derive(Debug, Clone, Copy, Default)]
pub struct BValidQuery<T>(pub T);

/// Validated Path extractor
#[derive(Debug, Clone, Copy, Default)]
pub struct BValidPath<T>(pub T);

/// Validated JSON extractor
#[derive(Debug, Clone, Copy, Default)]
pub struct BValidJson<T>(pub T);

// ===== Validation Trait Implementations =====

/// Implements HasValidate for BQuery wrapper
impl<T> HasValidate for BQuery<T> {
    type Validate = T;

    fn get_validate(&self) -> &Self::Validate {
        &self.0
    }
}

impl<T> HasValidate for BPath<T> {
    type Validate = T;
    fn get_validate(&self) -> &Self::Validate {
        &self.0
    }
}

impl<T> HasValidate for BJson<T> {
    type Validate = T;
    fn get_validate(&self) -> &Self::Validate {
        &self.0
    }
}

// ===== Macro for Implementing FromRequest Traits =====

/// Macro for implementing FromRequest/FromRequestParts traits
///
/// Generates boilerplate code for creating validated extractors by
/// composing existing extractors with the validation wrapper.
///
/// # Parameters
/// - `$name`: The target type name (e.g., BValidQuery)
/// - `$wrapper`: The base extractor wrapper (e.g., BQuery)
/// - `$trait_type`: The trait to implement (FromRequest or FromRequestParts)
macro_rules! impl_from_request {
    ($name:ident, $wrapper: ident, FromRequestParts) => {
        impl<S, T> FromRequestParts<S> for $name<T>
        where
            S: Send + Sync,
            BValid<$wrapper<T>>: FromRequestParts<S, Rejection = ApiError>,
        {
            type Rejection = ApiError;

            async fn from_request_parts(
                parts: &mut Parts,
                state: &S,
            ) -> Result<Self, Self::Rejection> {
                let result = BValid::from_request_parts(parts, state).await?;

                Ok($name(result.0 .0))
            }
        }
    };

    ($name:ident, $wrapper: ident, FromRequest) => {
        impl<S, T> FromRequest<S> for $name<T>
        where
            S: Send + Sync,
            BValid<$wrapper<T>>: FromRequest<S, Rejection = ApiError>,
        {
            type Rejection = ApiError;
            async fn from_request(request: Request, state: &S) -> Result<Self, Self::Rejection> {
                Ok($name(BValid::from_request(request, state).await?.0 .0))
            }
        }
    };
}

impl_from_request!(BValidQuery, BQuery, FromRequestParts);
impl_from_request!(BValidPath, BPath, FromRequest);
impl_from_request!(BValidJson, BJson, FromRequest);

// ===== Email Validation Utilities =====

/// The pattern validates standard email format according to common conventions.
static EMAIL_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^[a-zA-Z0-9_.+-]+@[a-zA-Z0-9-]+\.[a-zA-Z0-9-.]+$").expect("Invalid email regex")
});

/// Validates email address format
///
/// Checks if the provided string matches the standard email format pattern.
/// This is a basic format validation and does not verify email existence or deliverability.
pub fn is_email_valid(value: &str) -> Result<(), ValidationError> {
    if EMAIL_REGEX.is_match(value) {
        Ok(())
    } else {
        Err(ValidationError {
            code: Cow::from(""),
            message: Some(Cow::from("invalid email format.")),
            params: HashMap::new(),
        })
    }
}

use crate::auth::{get_jwt, Jwt};
use crate::error::ApiError;
use axum::body::Body;
use http::{header, Request, Response};
use std::future::Future;
use std::pin::Pin;
use std::sync::LazyLock;
use tower_http::auth::{AsyncAuthorizeRequest, AsyncRequireAuthorizationLayer};

static AUTH_LAYER: LazyLock<AsyncRequireAuthorizationLayer<JWTAuth>> =
    LazyLock::new(|| AsyncRequireAuthorizationLayer::new(JWTAuth::new(get_jwt())));

#[derive(Clone)]
pub struct JWTAuth {
    jwt: &'static Jwt,
}

impl JWTAuth {
    pub fn new(jwt: &'static Jwt) -> Self {
        Self { jwt }
    }
}

pub fn get_auth_layer() -> &'static AsyncRequireAuthorizationLayer<JWTAuth> {
    &AUTH_LAYER
}

impl AsyncAuthorizeRequest<Body> for JWTAuth {
    type RequestBody = Body;
    type ResponseBody = Body;
    type Future = Pin<
        Box<
            dyn Future<Output = Result<Request<Self::RequestBody>, Response<Self::ResponseBody>>>
                + Send
                + 'static,
        >,
    >;

    fn authorize(&mut self, mut request: Request<Self::RequestBody>) -> Self::Future {
        Box::pin(async {
            let token = request
                .headers()
                .get(header::AUTHORIZATION)
                .map(|value| -> Result<_, ApiError> {
                    let token_str = value
                        .to_str()
                        .map_err(|_| {
                            ApiError::UnAuthenticatedError(
                                "Authorization header is not a valid string".to_string(),
                            )
                        })?
                        .strip_prefix("Bearer ")
                        .ok_or_else(|| {
                            ApiError::UnAuthenticatedError(
                                "Authorization header must start with Bearer !".to_string(),
                            )
                        })?;

                    Ok(token_str)
                })
                .transpose()?
                .ok_or_else(|| {
                    ApiError::UnAuthenticatedError("Authorization header is not found!".to_string())
                })?;

            let principal = self.jwt.decode(token).map_err(|e| {
                tracing::error!("JWT decode error, Invalid token!: {:?}", e);
                ApiError::UnAuthenticatedError("Invalid token!".to_string())
            })?;
            request.extensions_mut().insert(principal);

            Ok(request)
        })
    }
}

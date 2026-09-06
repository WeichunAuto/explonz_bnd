use std::net::SocketAddr;

use axum::{
    debug_handler,
    extract::{ConnectInfo, Path, State},
    Json,
};
use explonz_shared::common::dto::LabelDto;

use crate::{
    api::labels::dto::CreateLabelRequest,
    application::AppState,
    error::ApiError,
    response::{ApiResponse, ApiResult},
    service::label::{create_label_service, delete_label_service, get_labels_service},
};

#[debug_handler]
#[tracing::instrument(name = "get_labels", skip_all)]
pub async fn get_labels(State(AppState { db, .. }): State<AppState>) -> ApiResult<Vec<LabelDto>> {
    let labels = get_labels_service(&db)
        .await
        .map_err(|e| ApiError::InternalError(e))?;
    Ok(ApiResponse::success("ok", Some(labels)))
}

// 创建 Label
#[debug_handler]
#[tracing::instrument(name = "create_label", skip_all, fields(IP = %addr))]
pub async fn create_label(
    State(AppState { db, .. }): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Json(label_request): Json<CreateLabelRequest>,
) -> ApiResult<LabelDto> {
    tracing::info!(
        "start create a label, address: {}, label name: {}",
        addr,
        &label_request.name
    );
    let label = create_label_service(&db, label_request)
        .await
        .map_err(|e| ApiError::InternalError(e))?;

    Ok(ApiResponse::success("spot created", Some(label)))
}

// 删除 Label
#[tracing::instrument(name = "delete_label", skip_all)]

pub async fn delete_label(
    State(AppState { db, .. }): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<()> {
    delete_label_service(&db, id)
        .await
        .map_err(|e| ApiError::InternalError(e))?;

    Ok(ApiResponse::success("OK", None))
}

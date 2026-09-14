use std::{net::SocketAddr, path::Path};

use axum::{
    debug_handler,
    extract::{ConnectInfo, Multipart, State},
    Json,
};
use explonz_shared::common::{
    dto::SpotDto,
    pagination::{Page, Pagination},
    utils::dir_into_url_path,
};
use serde::Deserialize;
use uuid::Uuid;
use validator::Validate;

use crate::{
    api::spots::dto::{CreateSpotRequest, ImageUploadResponse},
    application::AppState,
    error::ApiError,
    request::{BPath, BValidQuery},
    response::{ApiResponse, ApiResult},
    service::spots::{create_spot_service, get_spot_service, get_spots_service},
};

// Spot 查询条的件参数
#[derive(Debug, Deserialize, Validate)]
pub struct SpotQuery {
    pub id: Option<Uuid>,
    pub name: Option<String>,
    #[validate(nested)]
    #[serde(flatten)]
    pub pagination: Pagination,
}

#[debug_handler]
#[tracing::instrument(name = "create_spot", skip_all, fields(IP = %addr))]
pub async fn create_spot(
    State(AppState { db, .. }): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Json(spot_request): Json<CreateSpotRequest>,
) -> ApiResult<SpotDto> {
    tracing::info!(
        "start create a spot, address: {}, spot name: {}",
        addr,
        &spot_request.name
    );
    let spot = create_spot_service(&db, spot_request)
        .await
        .map_err(|e| ApiError::InternalError(e))?;

    Ok(ApiResponse::success("spot created", Some(spot)))
}

// 获取所有分页 Spots
#[debug_handler]
#[tracing::instrument(name = "get_spots", skip_all)]
pub async fn get_spots(
    State(AppState { db, .. }): State<AppState>,
    BValidQuery(spot_params): BValidQuery<SpotQuery>,
) -> ApiResult<Page<SpotDto>> {
    let spots_with_page = get_spots_service(&db, spot_params).await;
    Ok(ApiResponse::success("ok", Some(spots_with_page)))
}

// 获取某个 Spot
#[debug_handler]
#[tracing::instrument(name = "get_spot", skip_all)]
pub async fn get_spot(
    State(AppState { db, .. }): State<AppState>,
    BPath(spot_id): BPath<Uuid>,
) -> ApiResult<SpotDto> {
    let spot = get_spot_service(&db, spot_id)
        .await
        .map_err(ApiError::InternalError)?
        .ok_or(ApiError::NotFoundError)?;
    Ok(ApiResponse::success("ok", Some(spot)))
}

#[debug_handler]
#[tracing::instrument(name = "upload_image", skip_all, fields(IP = %addr))]
pub async fn upload_image(
    State(AppState {
        upload_dir,
        public_url,
        ..
    }): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    mut multipart: Multipart,
) -> ApiResult<ImageUploadResponse> {
    // tracing::info!("开始上传图片...");
    let upload_dir = format!("{}/spots/images", upload_dir);

    // 防止目录不存在
    tokio::fs::create_dir_all(&upload_dir)
        .await
        .map_err(|e| ApiError::InternalError(e.into()))?;

    let field = multipart
        .next_field()
        .await
        .map_err(|e| ApiError::InternalError(e.into()))?
        .ok_or_else(|| ApiError::BizError("No file provided.".into()))?;

    // 提取图片扩展名
    let ext = match field.content_type().unwrap_or("image/jpeg") {
        "image/jpeg" => "jpg",
        "image/png" => "png",
        "image/webp" => "webp",
        "image/gif" => "gif",
        _ => "jpg",
    };

    // 文件名
    let now = chrono::Local::now();
    let uploaded_file_name = format!("{}.{}", now.format("%Y%m%d%H%M%S%3f"), ext);

    // 图片数据
    let data = field
        .bytes()
        .await
        .map_err(|e| ApiError::InternalError(e.into()))?;

    // 写入路径
    let path = Path::new(&upload_dir).join(&uploaded_file_name);

    // 写入
    tokio::fs::write(path, data)
        .await
        .map_err(|e| ApiError::InternalError(e.into()))?;

    // 去掉开头的 "./" 或 "."，确保以 "/" 开头
    let upload_url_path = dir_into_url_path(&upload_dir);

    let url = format!("{}{}/{}", public_url, upload_url_path, uploaded_file_name);
    tracing::info!("image uploaded: {uploaded_file_name}");
    tracing::info!("image url: {url}");

    Ok(ApiResponse::success(
        "spot image uploaded.",
        Some(ImageUploadResponse {
            id: uploaded_file_name,
            url,
        }),
    ))
}

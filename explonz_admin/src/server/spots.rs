use crate::server::ApiResp;
use explonz_shared::common::{
    dto::{SeasonalPickingTypeDto, SeasonalPickingsDto, SpotDto}, pagination::Page,
};
use leptos::prelude::*;

// 获取 Spot 列表（分页 + 按 id/name 过滤）
#[server(GetSpots, "/api")]
pub async fn get_spots(
    id: Option<String>,
    name: Option<String>,
    page: u64,
    size: u64,
) -> Result<Page<SpotDto>, ServerFnError> {
    let token = crate::server::extract_token().await?;
    let backend_url = crate::server::backend_url();

    let mut params: Vec<(&str, String)> =
        vec![("page", page.to_string()), ("size", size.to_string())];
    if let Some(id_val) = id {
        params.push(("id", id_val));
    }
    if let Some(name_val) = name {
        params.push(("name", name_val));
    }

    let resp = reqwest::Client::new()
        .get(format!("{backend_url}/api/spots"))
        .query(&params)
        .bearer_auth(&token)
        .send()
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    if !resp.status().is_success() {
        let msg = resp.text().await.unwrap_or_default();
        return Err(ServerFnError::new(format!("Backend error: {msg}")));
    }

    let parsed: ApiResp<Page<SpotDto>> = resp
        .json()
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    parsed
        .data
        .ok_or_else(|| ServerFnError::new("No data returned"))
}

// 获取单个 Spot 详情
#[server(GetSpot, "/api")]
pub async fn get_spot(spot_id: String) -> Result<SpotDto, ServerFnError> {
    let token = crate::server::extract_token().await?;
    let backend_url = crate::server::backend_url();

    let resp = reqwest::Client::new()
        .get(format!("{backend_url}/api/spots/{spot_id}"))
        .bearer_auth(&token)
        .send()
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    if !resp.status().is_success() {
        let msg = resp.text().await.unwrap_or_default();
        return Err(ServerFnError::new(format!("Backend error: {msg}")));
    }

    let parsed: ApiResp<SpotDto> = resp
        .json()
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    parsed.data.ok_or_else(|| ServerFnError::new("Not found"))
}

// 删除 Spot
#[server(DeleteSpot, "/api")]
pub async fn delete_spot(spot_id: String) -> Result<(), ServerFnError> {
    let token = crate::server::extract_token().await?;
    let backend_url = crate::server::backend_url();

    let resp = reqwest::Client::new()
        .delete(format!("{backend_url}/api/spots/{spot_id}"))
        .bearer_auth(&token)
        .send()
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    if resp.status().is_success() {
        Ok(())
    } else {
        let msg = resp.text().await.unwrap_or_default();
        Err(ServerFnError::new(format!("Backend error: {msg}")))
    }
}

// 创建一个Spot
#[server(CreateSpot, "/api")]
pub async fn create_spot(
    name: String,
    location: String,
    latitude: f64,
    longitude: f64,
    description: String,
    photo_urls: Vec<String>, // 多个同名 input 直接反序列化为 Vec
    label_ids: Vec<String>,  // 选中的 label ID 列表，多个同名 hidden input
    phone: Option<String>,
    website: Option<String>,
    opening_hours_json: String, // 7天营业时间，JSON 序列化后传入。 // 由 UI 隐藏字段自动维护，见 addition.rs
) -> Result<SpotDto, ServerFnError> {
    use axum_extra::extract::CookieJar;
    use leptos_axum::extract;

    // 1. 取出 access_token cookie 作为 Bearer token
    let jar: CookieJar = extract().await?;
    let token = jar
        .get("access_token")
        .map(|c| c.value().to_string())
        .ok_or_else(|| ServerFnError::new("Not authenticated"))?;

    // 2. 过滤空值
    let photo_urls: Vec<String> = photo_urls
        .into_iter()
        .filter(|s| !s.trim().is_empty())
        .collect();

    // 3. 解析 opening_hours JSON，为空时默认 []
    let opening_hours: serde_json::Value = if opening_hours_json.trim().is_empty() {
        serde_json::Value::Array(vec![])
    } else {
        serde_json::from_str(&opening_hours_json)
            .map_err(|e| ServerFnError::new(format!("Invalid opening hours JSON: {e}")))?
    };

    // 4. 构造请求体
    let body = serde_json::json!({
        "name": name,
        "location": location,
        "latitude": latitude,
        "longitude": longitude,
        "description": description,
        "photo_urls": photo_urls,
        "label_ids": label_ids,
        "phone": phone,
        "website": website,
        "opening_hours": opening_hours,
    });

    // 5. 转发请求到后端（携带 Bearer token）
    let backend_url = crate::server::backend_url();

    let resp = reqwest::Client::new()
        .post(format!("{backend_url}/api/spots/new"))
        .bearer_auth(&token)
        .json(&body)
        .send()
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    // 6. 处理响应
    if !resp.status().is_success() {
        let msg = resp.text().await.unwrap_or_default();
        return Err(ServerFnError::new(format!("Backend error: {msg}")));
    }

    #[derive(serde::Deserialize)]
    struct BackendResponse {
        data: Option<SpotDto>,
    }

    let parsed: BackendResponse = resp
        .json()
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    parsed
        .data
        .ok_or_else(|| ServerFnError::new("No data returned"))
}

// 更新一个Spot
#[server(UpdateSpot, "/api")]
pub async fn update_spot(
    spot_id: String,
    name: String,
    location: String,
    latitude: f64,
    longitude: f64,
    description: String,
    photo_urls: Vec<String>,
    label_ids: Vec<String>,
    phone: Option<String>,
    website: Option<String>,
    opening_hours_json: String,
) -> Result<SpotDto, ServerFnError> {
    let token = crate::server::extract_token().await?;

    let photo_urls: Vec<String> = photo_urls
        .into_iter()
        .filter(|s| !s.trim().is_empty())
        .collect();

    let opening_hours: serde_json::Value = if opening_hours_json.trim().is_empty() {
        serde_json::Value::Array(vec![])
    } else {
        serde_json::from_str(&opening_hours_json)
            .map_err(|e| ServerFnError::new(format!("Invalid opening hours JSON: {e}")))?
    };

    let body = serde_json::json!({
        "name": name,
        "location": location,
        "latitude": latitude,
        "longitude": longitude,
        "description": description,
        "photo_urls": photo_urls,
        "label_ids": label_ids,
        "phone": phone,
        "website": website,
        "opening_hours": opening_hours,
    });

    let backend_url = crate::server::backend_url();

    let resp = reqwest::Client::new()
        .put(format!("{backend_url}/api/spots/{spot_id}"))
        .bearer_auth(&token)
        .json(&body)
        .send()
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    if !resp.status().is_success() {
        let msg = resp.text().await.unwrap_or_default();
        return Err(ServerFnError::new(format!("Backend error: {msg}")));
    }

    let parsed: ApiResp<SpotDto> = resp
        .json()
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    parsed
        .data
        .ok_or_else(|| ServerFnError::new("No data returned"))
}

// 为 Spot 创建一条 SeasonalPicking
#[server(CreateSeasonalPicking, "/api")]
pub async fn create_seasonal_picking(
    spot_id: String,
    type_id: String,
    season_start_month: i16,
    season_start_day: i16,
    season_end_month: i16,
    season_end_day: i16,
) -> Result<(), ServerFnError> {
    let token = crate::server::extract_token().await?;
    let backend_url = crate::server::backend_url();

    let body = serde_json::json!({
        "spot_id": spot_id,
        "type_id": type_id,
        "start_month": season_start_month,
        "start_day": season_start_day,
        "end_month": season_end_month,
        "end_day": season_end_day,
    });

    let resp = reqwest::Client::new()
        .post(format!(
            "{backend_url}/api/spots/{spot_id}/seasonal_pickings/new"
        ))
        .bearer_auth(&token)
        .json(&body)
        .send()
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    if resp.status().is_success() {
        Ok(())
    } else {
        let msg = resp.text().await.unwrap_or_default();
        Err(ServerFnError::new(format!("Backend error: {msg}")))
    }
}

// 获取某个 spot 的 seasonal pickings
#[server(GetSeasonalPickings, "/api")]
pub async fn get_seasonal_pickings(spot_id: String,) -> Result<Vec<SeasonalPickingsDto>, ServerFnError> {
    let token = crate::server::extract_token().await?;
    let backend_url = crate::server::backend_url();

    let resp = reqwest::Client::new()
        .get(format!("{backend_url}/api/spots/{spot_id}/seasonal_pickings"))
        .bearer_auth(&token)
        .send()
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    if !resp.status().is_success() {
        let msg = resp.text().await.unwrap_or_default();
        return Err(ServerFnError::new(format!("Backend error: {msg}")));
    }

    let parsed: ApiResp<Vec<SeasonalPickingsDto>> = resp
        .json()
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    parsed
        .data
        .ok_or_else(|| ServerFnError::new("No data returned"))
}

// 删除一条 SeasonalPicking
#[server(DeleteSeasonalPicking, "/api")]
pub async fn delete_seasonal_picking(picking_id: String) -> Result<(), ServerFnError> {
    let token = crate::server::extract_token().await?;
    let backend_url = crate::server::backend_url();

    let resp = reqwest::Client::new()
        .delete(format!(
            "{backend_url}/api/spots/seasonal_pickings/{picking_id}"
        ))
        .bearer_auth(&token)
        .send()
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    if resp.status().is_success() {
        Ok(())
    } else {
        let msg = resp.text().await.unwrap_or_default();
        Err(ServerFnError::new(format!("Backend error: {msg}")))
    }
}

// 获取 所有的seasonal_picking_types
#[server(GetSeasonalPickingTypes, "/api")]
pub async fn get_seasonal_picking_types() -> Result<Vec<SeasonalPickingTypeDto>, ServerFnError> {
    let token = crate::server::extract_token().await?;
    let backend_url = crate::server::backend_url();

    let resp = reqwest::Client::new()
        .get(format!("{backend_url}/api/spots/seasonal_picking_types"))
        .bearer_auth(&token)
        .send()
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    if !resp.status().is_success() {
        let msg = resp.text().await.unwrap_or_default();
        return Err(ServerFnError::new(format!("Backend error: {msg}")));
    }

    let parsed: ApiResp<Vec<SeasonalPickingTypeDto>> = resp
        .json()
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    parsed
        .data
        .ok_or_else(|| ServerFnError::new("No data returned"))
}

#[server(GeocodeLocation, "/api")]
pub async fn geocode_location(address: String) -> Result<(f64, f64), ServerFnError> {
    use serde::Deserialize;

    #[derive(Deserialize)]
    struct NominatimResult {
        lat: String,
        lon: String,
    }

    let results: Vec<NominatimResult> = reqwest::Client::new()
        .get("https://nominatim.openstreetmap.org/search")
        .query(&[("q", address.as_str()), ("format", "json"), ("limit", "1")])
        .header("User-Agent", "explonz-admin/1.0")
        .send()
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?
        .json()
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    let result = results
        .into_iter()
        .next()
        .ok_or_else(|| ServerFnError::new("No location found for this address"))?;

    Ok((
        result
            .lat
            .parse::<f64>()
            .map_err(|e| ServerFnError::new(e.to_string()))?,
        result
            .lon
            .parse::<f64>()
            .map_err(|e| ServerFnError::new(e.to_string()))?,
    ))
}

/// 图片上传结果，供 addition.rs 中的 PhotoStatus::Done 使用
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PhotoUploadResponse {
    pub id: String,  // 后端返回的文件名，用于删除
    pub url: String, // 公开访问地址，写入 photo_urls
}

/// 上传图片到后端
/// input = MultipartFormData：客户端发送 FormData，服务端接收 Multipart
#[server(UploadPhoto, "/api", input = server_fn::codec::MultipartFormData)]
pub async fn upload_photo(
    data: server_fn::codec::MultipartData,
) -> Result<PhotoUploadResponse, ServerFnError> {
    let token = crate::server::extract_token().await?;

    // server 端：into_inner() 返回 Some(axum::extract::Multipart)
    let mut multipart = data
        .into_inner()
        .ok_or_else(|| ServerFnError::new("No multipart data"))?;

    while let Ok(Some(field)) = multipart.next_field().await {
        if field.name() != Some("file") {
            continue;
        }
        let filename = field.file_name().unwrap_or("upload").to_string();

        let content_type = field
            .content_type()
            .map(|m| m.to_string())
            .unwrap_or_else(|| "image/jpeg".to_string());
        let bytes = field
            .bytes()
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;

        // 构造 multipart 转发给后端
        let part = reqwest::multipart::Part::bytes(bytes.to_vec())
            .file_name(filename)
            .mime_str(&content_type)
            .map_err(|e| ServerFnError::new(e.to_string()))?;

        let form = reqwest::multipart::Form::new().part("file", part);

        let backend_url =
            std::env::var("BACKEND_URL").unwrap_or_else(|_| "http://127.0.0.1:3000".to_string());

        let resp = reqwest::Client::new()
            .post(format!("{backend_url}/api/spots/images"))
            .bearer_auth(&token)
            .multipart(form)
            .send()
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;

        if !resp.status().is_success() {
            let msg = resp.text().await.unwrap_or_default();
            return Err(ServerFnError::new(format!("Backend error: {msg}")));
        }

        let parsed: ApiResp<PhotoUploadResponse> = resp
            .json()
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;

        return parsed
            .data
            .ok_or_else(|| ServerFnError::new("No data from backend"));
    }

    Err(ServerFnError::new("No file field found"))
}

/// 删除图片（无需通知后端删除本地文件，删除图片操作将由清理任务自动删除）
#[server(DeletePhoto, "/api")]
pub async fn delete_photo(img_id: String) -> Result<(), ServerFnError> {
    Ok(())
}

use explonz_shared::common::dto::SpotDto;
use leptos::prelude::*;

#[server(GetSpots, "/api")]
pub async fn get_spots(page: u64, page_size: u64) -> Result<Vec<SpotDto>, ServerFnError> {
    use explonz_shared::entity::spots;
    use sea_orm::{DatabaseConnection, EntityTrait};

    let db = use_context::<DatabaseConnection>()
        .ok_or_else(|| ServerFnError::new("No DB connection"))?;
    let models = spots::Entity::find()
        .all(&db)
        .await
        .map_err(|e| ServerFnError::new(e))?;
    Ok(models.into_iter().map(SpotDto::from).collect())
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
    attributes_json: String,
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

    // 3. 解析 attributes JSON，为空时默认 []
    let attributes: serde_json::Value = if attributes_json.trim().is_empty() {
        serde_json::Value::Array(vec![])
    } else {
        serde_json::from_str(&attributes_json)
            .map_err(|e| ServerFnError::new(format!("Invalid JSON: {e}")))?
    };

    // 4. 解析 opening_hours JSON，为空时默认 []
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
        "attributes": attributes,
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
        println!("field name = {:?}", field.name());

        if field.name() != Some("file") {
            continue;
        }
        let filename = field.file_name().unwrap_or("upload").to_string();

        // println!("filename = {:?}", filename);
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

        #[derive(serde::Deserialize)]
        struct BackendResp {
            data: Option<PhotoUploadResponse>,
        }

        let parsed: BackendResp = resp
            .json()
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;

        return parsed
            .data
            .ok_or_else(|| ServerFnError::new("No data from backend"));
    }

    Err(ServerFnError::new("No file field found"))
}

/// 删除图片（通知后端删除本地文件）
#[server(DeletePhoto, "/api")]
pub async fn delete_photo(img_id: String) -> Result<(), ServerFnError> {
    let token = crate::server::extract_token().await?;

    let backend_url = crate::server::backend_url();

    let resp = reqwest::Client::new()
        .delete(format!("{backend_url}/api/images/{img_id}"))
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

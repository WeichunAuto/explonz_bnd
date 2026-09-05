use explonz_shared::common::dto::LabelDto;
use leptos::prelude::*;

// ---------------------------------------------------------------------------
// GET — 直接查 DB（与 get_spots 模式一致）
// ---------------------------------------------------------------------------

#[server(GetLabels, "/api")]
pub async fn get_labels() -> Result<Vec<LabelDto>, ServerFnError> {
    let token = crate::server::extract_token().await?;
    let backend_url = crate::server::backend_url();

    let resp = reqwest::Client::new()
        .get(format!("{backend_url}/api/labels"))
        .bearer_auth(&token)
        .send()
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    if !resp.status().is_success() {
        let msg = resp.text().await.unwrap_or_default();
        return Err(ServerFnError::new(format!("Backend error: {msg}")));
    }

    let parsed: crate::server::ApiResp<Vec<LabelDto>> = resp
        .json()
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    Ok(parsed.data.unwrap_or_default())
}

// ---------------------------------------------------------------------------
// CREATE — 转发到后端 API
// ---------------------------------------------------------------------------

#[server(CreateLabel, "/api")]
pub async fn create_label(
    name: String,
    description: String,
    icon: String,
) -> Result<LabelDto, ServerFnError> {
    let token = crate::server::extract_token().await?;

    let backend_url = crate::server::backend_url();

    let body = serde_json::json!({
        "name":        name,
        "description": description,
        "icon":        icon,
    });

    let resp = reqwest::Client::new()
        .post(format!("{backend_url}/api/labels/new"))
        .bearer_auth(&token)
        .json(&body)
        .send()
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    if !resp.status().is_success() {
        let msg = resp.text().await.unwrap_or_default();
        return Err(ServerFnError::new(format!("Backend error: {msg}")));
    }

    let parsed: crate::server::ApiResp<LabelDto> = resp
        .json()
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    parsed
        .data
        .ok_or_else(|| ServerFnError::new("No data returned"))
}

// ---------------------------------------------------------------------------
// UPDATE — 转发到后端 API
// ---------------------------------------------------------------------------
#[server(UpdateLabel, "/api")]
pub async fn update_label(
    id: String,
    name: String,
    description: String,
    icon: String,
) -> Result<LabelDto, ServerFnError> {
    let token = crate::server::extract_token().await?;

    let backend_url = crate::server::backend_url();

    let body = serde_json::json!({
        "name":        name,
        "description": description,
        "icon":        icon,
    });

    let resp = reqwest::Client::new()
        .put(format!("{backend_url}/api/labels/{id}"))
        .bearer_auth(&token)
        .json(&body)
        .send()
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    if !resp.status().is_success() {
        let msg = resp.text().await.unwrap_or_default();
        return Err(ServerFnError::new(format!("Backend error: {msg}")));
    }

    let parsed: crate::server::ApiResp<LabelDto> = resp
        .json()
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    parsed
        .data
        .ok_or_else(|| ServerFnError::new("No data returned"))
}

// ---------------------------------------------------------------------------
// DELETE — 转发到后端 API
// ---------------------------------------------------------------------------
#[server(DeleteLabel, "/api")]
pub async fn delete_label(id: String) -> Result<(), ServerFnError> {
    let token = crate::server::extract_token().await?;

    let backend_url = crate::server::backend_url();

    let resp = reqwest::Client::new()
        .delete(format!("{backend_url}/api/labels/{id}"))
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

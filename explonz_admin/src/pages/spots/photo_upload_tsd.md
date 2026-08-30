# TSD：Spot 图片上传

> 本文档替代 `addition_tsd.md` 和 `add_opentime_contact_tsd.md` 中关于 photo_urls 的章节。

---

## 一、整体架构

图片直接存储到后端服务器本地目录，通过 Axum `ServeDir` 对外提供访问。
上传和删除均通过 Leptos server fn 实现，**无需在 Admin 增加自定义 Axum 路由**。

```
┌─────────────────────────────────────────────────────────────┐
│  浏览器（WASM）                                              │
│  用户选择/拖入文件                                           │
│    → 构造 web_sys::FormData                                  │
│    → 调用 upload_photo server fn（MultipartFormData）        │
│    ← 返回 PhotoUploadResponse { id, url }，实时渲染预览      │
│                                                              │
│  用户点删除                                                  │
│    → 调用 delete_photo server fn（img_id: String）           │
│                                                              │
│  提交表单                                                    │
│    → photo_urls[] 由 Done 状态的隐藏 input 自动序列化        │
└──────────────────────┬──────────────────────────────────────┘
         Leptos server fn 机制（SSR 端执行）
┌──────────────────────▼──────────────────────────────────────┐
│  Admin SSR（explonz_admin）                                  │
│  upload_photo server fn                                      │
│    → 读取 MultipartData → reqwest POST /api/images           │
│  delete_photo server fn                                      │
│    → reqwest DELETE /api/images/:id                          │
│  （access_token cookie 作为 Bearer token 转发）              │
└──────────────────────┬──────────────────────────────────────┘
                       │ HTTP (reqwest，Bearer token)
┌──────────────────────▼──────────────────────────────────────┐
│  后端 (explonz_bnd)                                          │
│  POST   /api/images       → 写入本地 UPLOAD_DIR              │
│  DELETE /api/images/:id   → 删除本地文件                     │
│  GET    /static/images/*  → ServeDir 静态文件服务            │
│                                                              │
│  background task（Tokio）                                    │
│  扫描 UPLOAD_DIR → 对比 DB photo_urls → 删除孤立文件         │
└─────────────────────────────────────────────────────────────┘
```

**设计要点：**
- 无需在 Admin 新增 Axum 路由，无需 `gloo-net` 等 WASM HTTP 库
- 上传/删除逻辑在 SSR 端执行，`reqwest` 调用后端 API
- WASM 端仅负责读取文件构造 `FormData`，通过 server fn 机制传递
- 清理任务直接用 `tokio::fs` 扫描本地目录，无外部 HTTP 调用
- 图片 URL 格式：`{PUBLIC_URL}/static/images/{filename}`

---

## 二、环境变量

### `explonz_bnd/.env`（后端，新增三行）

```dotenv
DATABASE_URL=postgres://wangweichun:WW_33_cc@localhost:5432/explonz_app  # 已有

UPLOAD_DIR=./uploads                    # 图片存储目录（相对于后端工作目录）
PUBLIC_URL=http://192.168.68.50:3005    # 后端对外地址，用于拼接图片 URL
IMAGE_CLEANUP_INTERVAL_SECS=3600        # 清理任务执行间隔（秒），可选，默认 3600
IMAGE_CLEANUP_GRACE_SECS=1800           # 孤立图片宽限期（秒），可选，默认 1800
```

### `explonz_admin/.env`（Admin，无需变更）

```dotenv
# 已有配置，server fn 内 reqwest 调用后端时使用，无需新增变量
ADMIN_ACCOUNT=bobby
ADMIN_PASSWORD_HASH=$2b$12$...
BACKEND_URL=http://192.168.68.50:3005
```

> `UPLOAD_DIR` 默认值为 `./uploads`，后端启动时会自动 `create_dir_all` 创建该目录，无需手动创建。
> `PUBLIC_URL` 与 `BACKEND_URL` 通常填同一个地址，前者供后端生成图片 URL，后者供 Admin 调用后端 API。

---

## 三、后端变更（explonz_bnd）

### 3.1 `AppState` 新增字段

```rust
#[derive(Clone)]
pub struct AppState {
    pub db: DatabaseConnection,
    pub upload_dir: String, // 新增
    pub public_url: String, // 新增
}
```

从环境变量初始化：
```rust
let upload_dir = std::env::var("UPLOAD_DIR").unwrap_or_else(|_| "./uploads".to_string());
let public_url = std::env::var("PUBLIC_URL").unwrap_or_else(|_| "http://127.0.0.1:3000".to_string());
// 确保目录存在
tokio::fs::create_dir_all(&upload_dir).await?;
```

### 3.2 新增 `src/api/images/handler.rs`

```rust
use axum::{
    extract::{Multipart, Path, State},
    http::StatusCode,
};
use uuid::Uuid;

use crate::{
    application::AppState,
    error::ApiError,
    response::{ApiResponse, ApiResult},
};

#[derive(serde::Serialize, serde::Deserialize)]
pub struct ImageUploadResponse {
    pub id: String,  // 文件名，删除时使用
    pub url: String, // 完整公开访问 URL
}

pub async fn upload_image(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> ApiResult<ImageUploadResponse> {
    let field = multipart
        .next_field()
        .await
        .map_err(|e| ApiError::InternalError(e.into()))?
        .ok_or_else(|| ApiError::BadRequestError("No file provided".into()))?;

    // 根据 content-type 推断扩展名
    let ext = match field.content_type().unwrap_or("image/jpeg") {
        "image/jpeg" => "jpg",
        "image/png"  => "png",
        "image/webp" => "webp",
        "image/gif"  => "gif",
        _            => "jpg",
    };
    let filename = format!("{}.{}", Uuid::new_v4(), ext);

    let data = field
        .bytes()
        .await
        .map_err(|e| ApiError::InternalError(e.into()))?;

    // 防止路径穿越（filename 由 Uuid 生成，此处仅作防御性检查）
    if filename.contains('/') || filename.contains('\\') || filename.contains("..") {
        return Err(ApiError::BadRequestError("Invalid filename".into()));
    }

    let path = std::path::Path::new(&state.upload_dir).join(&filename);
    tokio::fs::write(&path, &data)
        .await
        .map_err(|e| ApiError::InternalError(e.into()))?;

    let url = format!("{}/static/images/{}", state.public_url, filename);
    tracing::info!("image uploaded: {filename}");

    Ok(ApiResponse::success("uploaded", Some(ImageUploadResponse { id: filename, url })))
}

pub async fn delete_image(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<StatusCode, ApiError> {
    // 防止路径穿越
    if id.contains('/') || id.contains('\\') || id.contains("..") {
        return Err(ApiError::BadRequestError("Invalid id".into()));
    }

    let path = std::path::Path::new(&state.upload_dir).join(&id);
    tokio::fs::remove_file(&path)
        .await
        .map_err(|_| ApiError::NotFoundError)?;

    tracing::info!("image deleted: {id}");
    Ok(StatusCode::NO_CONTENT)
}
```

### 3.3 新增 `src/api/images/mod.rs`

```rust
pub mod handler;

use axum::{routing::{delete, post}, Router};
use crate::application::AppState;
use handler::{delete_image, upload_image};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/images", post(upload_image))
        .route("/images/:id", delete(delete_image))
}
```

### 3.4 `src/api/mod.rs` 注册路由 + 静态文件服务

```rust
use tower_http::services::ServeDir;

pub async fn build_routes(upload_dir: String) -> Router<AppState> {
    Router::new()
        .route("/", get(index))
        .nest("/api", user::routes())
        .nest("/api", spots::routes())
        .nest("/api", images::routes())   // 新增
        .route_layer(get_auth_layer())
        .nest("/auth", auth::routes())
        // 静态文件服务，无需鉴权
        .nest_service("/static/images", ServeDir::new(&upload_dir))
        .fallback(fallback)
        .method_not_allowed_fallback(async || -> ApiError {
            ApiError::MethodNotAllowedError
        })
}
```

> `tower-http` 已在 workspace 依赖中，`ServeDir` 需开启 `fs` feature：
> ```toml
> tower-http = { version = "0.6.6", features = ["trace", "timeout", "limit", "cors",
>                                                "normalize-path", "auth", "fs"] }
> ```

---

## 四、Admin Server Fn（图片上传/删除）

**不再需要自定义 Axum 路由**，图片上传和删除均通过 Leptos server fn 实现：

- 浏览器（WASM）调用 server fn → server fn 在 SSR 端用 `reqwest` 调用后端 API
- 删除使用普通 server fn（参数为 `img_id: String`）
- 上传使用 `MultipartFormData` 编码的 server fn，接收浏览器的 `FormData`

**与原方案对比：**

| | 原方案（自定义 Axum 路由） | 新方案（server fn） |
|---|---|---|
| Admin 路由 | `POST/DELETE /admin/api/photos` | 无，复用 `/api` server fn 路由 |
| 浏览器 HTTP 库 | `gloo-net` | 无（server fn 内置机制） |
| 额外依赖 | `gloo-net`, `wasm-bindgen-futures` | 无新增 |
| 额外文件 | `src/api/photos.rs` | 无 |

### 4.1 `explonz_admin/src/server/spots.rs` — 新增 `upload_photo` 和 `delete_photo`

在现有 `server/spots.rs` 末尾追加：

```rust
/// 图片上传结果，供 addition.rs 中的 PhotoStatus::Done 使用
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PhotoUploadResponse {
    pub id:  String, // 后端返回的文件名，用于删除
    pub url: String, // 公开访问地址，写入 photo_urls
}

/// 上传图片到后端
/// input = MultipartFormData：客户端发送 FormData，服务端接收 Multipart
/// [FIX] Leptos 0.8 不再支持字符串形式 "MultipartFormData"，改用类型路径
#[server(UploadPhoto, "/api", input = server_fn::codec::MultipartFormData)]
pub async fn upload_photo(
    data: server_fn::codec::MultipartData,
) -> Result<PhotoUploadResponse, ServerFnError> {
    use axum_extra::extract::CookieJar;
    use leptos_axum::extract;

    let jar: CookieJar = extract().await?;
    let token = jar
        .get("access_token")
        .map(|c| c.value().to_string())
        .ok_or_else(|| ServerFnError::new("Not authenticated"))?;

    // server 端：into_inner() 返回 Some(axum::extract::Multipart)
    let mut multipart = data
        .into_inner()
        .ok_or_else(|| ServerFnError::new("No multipart data"))?;

    while let Ok(Some(field)) = multipart.next_field().await {
        if field.name() != Some("file") {
            continue;
        }
        let filename     = field.file_name().unwrap_or("upload").to_string();
        // [FIX] content_type() 返回 Option<&Mime>，不能直接 unwrap_or(&str)
        let content_type = field.content_type()
            .map(|m| m.to_string())
            .unwrap_or_else(|| "image/jpeg".to_string());
        let bytes        = field.bytes().await
            .map_err(|e| ServerFnError::new(e.to_string()))?;

        // 构造 multipart 转发给后端
        let part = reqwest::multipart::Part::bytes(bytes.to_vec())
            .file_name(filename)
            .mime_str(&content_type)
            .map_err(|e| ServerFnError::new(e.to_string()))?;
        let form = reqwest::multipart::Form::new().part("file", part);

        let backend_url = std::env::var("BACKEND_URL")
            .unwrap_or_else(|_| "http://127.0.0.1:3000".to_string());

        let resp = reqwest::Client::new()
            .post(format!("{backend_url}/api/images"))
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
        struct BackendResp { data: Option<PhotoUploadResponse> }

        let parsed: BackendResp = resp.json().await
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
    use axum_extra::extract::CookieJar;
    use leptos_axum::extract;

    let jar: CookieJar = extract().await?;
    let token = jar
        .get("access_token")
        .map(|c| c.value().to_string())
        .ok_or_else(|| ServerFnError::new("Not authenticated"))?;

    let backend_url = std::env::var("BACKEND_URL")
        .unwrap_or_else(|_| "http://127.0.0.1:3000".to_string());

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
```

---

## 五、`explonz_admin/src/pages/spots/addition.rs`（完整代码，含变更标注）

变更标注约定：
- `// [NEW]` — 新增代码
- `// [CHANGED] was: ...` — 替换原有代码
- `// [UNCHANGED]` — 未改动，保持原样

**WASM 隔离说明**：`web_sys::FileList`、`web_sys::FormData`、`gloo_net` 均为 WASM-only API，
所有调用点均用 `#[cfg(target_arch = "wasm32")]` 隔离，确保 SSR build 正常编译。

```rust
// [CHANGED] was: use icons::Trash2;
// now: 新增 CloudUpload 图标用于 Dropzone
use icons::{Trash2, CloudUpload};
use leptos::prelude::*;
use leptos_router::hooks::use_navigate;
use leptos_router::NavigateOptions;
use serde::Serialize;

use crate::components::ui::button::{Button, ButtonVariant};
use crate::components::ui::card::{Card, CardContent, CardHeader, CardTitle};
use crate::components::ui::input::{Input, InputType};
use crate::components::ui::label::Label;
use crate::server::spots::{geocode_location, CreateSpot};

// [UNCHANGED]
const TEXTAREA_CLASS: &str = "text-foreground placeholder:text-muted-foreground border-input \
    flex w-full rounded-md border bg-transparent px-3 py-2 text-sm shadow-xs outline-none \
    focus-visible:border-ring focus-visible:ring-ring/50 focus-visible:ring-2 \
    dark:bg-input/30 resize-none";

// [UNCHANGED]
#[derive(Clone, PartialEq)]
enum DayStatus {
    Open,
    Closed,
    Open24h,
}

// [UNCHANGED]
#[derive(Clone)]
struct DaySchedule {
    status: RwSignal<DayStatus>,
    open_time: RwSignal<String>,
    close_time: RwSignal<String>,
}

// [UNCHANGED]
#[derive(Serialize)]
struct OpeningHourJson {
    day_of_week: i16,
    is_closed: bool,
    is_open_24h: bool,
    open_time: Option<String>,
    close_time: Option<String>,
}

// ── [NEW] 图片上传状态类型 ──────────────────────────────────────

/// 单张图片的本地状态，id 作为 For key 保持稳定
#[derive(Clone)]
struct PhotoItem {
    id: u32,
    status: RwSignal<PhotoStatus>,
}

/// 图片上传状态机
#[derive(Clone)]
enum PhotoStatus {
    Uploading,
    /// img_id: 后端返回的文件名（用于删除）；url: 公开访问地址（写入隐藏字段）
    Done { img_id: String, url: String },
    Failed(String),
}

// ── [NEW] WASM-only 上传辅助函数 ───────────────────────────────
/// 将单个文件通过 upload_photo server fn 上传，异步更新 status signal
/// [CHANGED] was: 直接用 gloo_net::http::Request::post("/admin/api/photos")
/// now: 调用 upload_photo server fn（服务端再转发给后端 API）
// ⚠️ rust-analyzer 提示 "code is inactive"：因 rust-analyzer 以 native 目标分析，
//    wasm32 cfg 块均显示灰色。cargo leptos build 时以 wasm32 编译，此函数会正常生效。
#[cfg(target_arch = "wasm32")]
fn upload_file(
    file: web_sys::File,
    status: RwSignal<PhotoStatus>,
) {
    use crate::server::spots::upload_photo;
    use server_fn::codec::MultipartData;

    leptos::task::spawn_local(async move {
        // 构造 FormData，server fn MultipartFormData 编码会将其序列化后发送
        let form_data = web_sys::FormData::new().unwrap();
        let _ = form_data.append_with_blob("file", &file);

        match upload_photo(MultipartData::from(form_data)).await {
            Ok(resp) => status.set(PhotoStatus::Done {
                img_id: resp.id,
                url:    resp.url,
            }),
            // [FIX] ServerFnError 是泛型类型，e.to_string() 会触发 E0282 类型推断歧义
            //       改用 format!("{e}") 绕过泛型参数推断
            Err(e) => status.set(PhotoStatus::Failed(format!("{e}"))),
        }
    });
}

// ── [NEW] WASM-only 批量处理 FileList ──────────────────────────
// ⚠️ rust-analyzer inactive（同上，wasm32 build 正常编译）
#[cfg(target_arch = "wasm32")]
fn process_files(
    files: web_sys::FileList,
    next_id: RwSignal<u32>,
    photos: RwSignal<Vec<PhotoItem>>,
) {
    for i in 0..files.length() {
        if let Some(file) = files.item(i) {
            let local_id = next_id.get_untracked();
            next_id.update(|n| *n += 1);
            let status = RwSignal::new(PhotoStatus::Uploading);
            photos.update(|v| v.push(PhotoItem { id: local_id, status }));
            // [CHANGED] was: upload_file(file, status, photos, local_id)
            // now: photos 参数移除（upload_file 只更新 status，删除由外部 photos signal 处理）
            upload_file(file, status);
        }
    }
}

// ───────────────────────────────────────────────────────────────

#[component]
pub fn SpotAddition() -> impl IntoView {
    let create_action = ServerAction::<CreateSpot>::new();
    let navigate = use_navigate();
    let navigate_cancel = navigate.clone();

    // [CHANGED] was: next_id = RwSignal::new(1u32) + photo_rows: RwSignal<Vec<(u32, RwSignal<String>)>>
    // now: photos list with PhotoItem/PhotoStatus state machine
    let next_id = RwSignal::new(0u32);
    let photos: RwSignal<Vec<PhotoItem>> = RwSignal::new(vec![]);

    // [NEW] Dropzone 拖拽高亮状态
    let drag_over = RwSignal::new(false);

    // [NEW] 隐藏 file input 的节点引用，点击 Dropzone 区域时触发它
    let file_input_ref = NodeRef::<leptos::html::Input>::new();

    // [UNCHANGED] Location / geocode
    let location_val = RwSignal::new(String::new());
    let lat = RwSignal::new(String::new());
    let lng = RwSignal::new(String::new());

    let geocode_action = Action::new(move |address: &String| {
        let address = address.clone();
        async move { geocode_location(address).await }
    });

    Effect::new(move |_| {
        if let Some(Ok((lat_val, lng_val))) = geocode_action.value().get() {
            lat.set(format!("{lat_val}"));
            lng.set(format!("{lng_val}"));
        }
    });

    // [UNCHANGED] 7 天营业时间
    let days: [(&'static str, DayStatus); 7] = [
        ("Sunday",    DayStatus::Closed),
        ("Monday",    DayStatus::Open),
        ("Tuesday",   DayStatus::Open),
        ("Wednesday", DayStatus::Open),
        ("Thursday",  DayStatus::Open),
        ("Friday",    DayStatus::Open),
        ("Saturday",  DayStatus::Closed),
    ];
    let schedules: Vec<(&'static str, DaySchedule)> = days
        .into_iter()
        .map(|(name, default_status)| {
            (name, DaySchedule {
                status: RwSignal::new(default_status),
                open_time: RwSignal::new("09:00".to_string()),
                close_time: RwSignal::new("17:00".to_string()),
            })
        })
        .collect();
    let schedules = StoredValue::new(schedules);

    // [UNCHANGED] opening hours memo
    let hours_memo = Memo::new(move |_| {
        let entries: Vec<OpeningHourJson> = schedules
            .get_value()
            .iter()
            .enumerate()
            .map(|(i, (_, s))| {
                let status = s.status.get();
                OpeningHourJson {
                    day_of_week: i as i16,
                    is_closed: status == DayStatus::Closed,
                    is_open_24h: status == DayStatus::Open24h,
                    open_time: if status == DayStatus::Open { Some(s.open_time.get()) } else { None },
                    close_time: if status == DayStatus::Open { Some(s.close_time.get()) } else { None },
                }
            })
            .collect();
        serde_json::to_string(&entries).unwrap_or_default()
    });

    // [UNCHANGED] 提交成功后跳转
    Effect::new(move |_| {
        if let Some(Ok(_)) = create_action.value().get() {
            navigate("/spots", NavigateOptions::default());
        }
    });

    view! {
        <div class="p-6 mx-auto">
            <h1 class="text-2xl font-semibold mb-6">"Create Spot"</h1>

            <Card>
                <CardHeader>
                    <CardTitle>"Spot Details"</CardTitle>
                </CardHeader>
                <CardContent>
                    <ActionForm action=create_action>
                        <div class="flex flex-col gap-5">

                            // [UNCHANGED] Name
                            <div class="grid gap-2">
                                <Label html_for="name">"Name"</Label>
                                <Input id="name" name="name"
                                    placeholder="e.g. Kumeu Orchard" required=true />
                            </div>

                            // [UNCHANGED] Location + Lookup
                            <div class="grid gap-2">
                                <Label html_for="location">"Location"</Label>
                                <div class="flex gap-2">
                                    <Input
                                        id="location"
                                        name="location"
                                        bind_value=location_val
                                        placeholder="e.g. Kumeu, Auckland"
                                        required=true
                                    />
                                    <Button
                                        variant=ButtonVariant::Outline
                                        attr:disabled=move || geocode_action.pending().get()
                                        on:click=move |_| {
                                            geocode_action.dispatch(location_val.get_untracked());
                                        }
                                    >
                                        {move || if geocode_action.pending().get() {
                                            "Looking up..."
                                        } else {
                                            "Lookup"
                                        }}
                                    </Button>
                                </div>
                                {move || geocode_action.value().get()
                                    .and_then(|r| r.err())
                                    .map(|e| view! {
                                        <p class="text-xs text-destructive">{e.to_string()}</p>
                                    })
                                }
                            </div>

                            // [UNCHANGED] Latitude & Longitude
                            <div class="grid grid-cols-2 gap-4">
                                <div class="grid gap-2">
                                    <Label html_for="latitude">"Latitude"</Label>
                                    <Input r#type=InputType::Number id="latitude" name="latitude"
                                        placeholder="-36.7896" step="any" required=true bind_value=lat />
                                </div>
                                <div class="grid gap-2">
                                    <Label html_for="longitude">"Longitude"</Label>
                                    <Input r#type=InputType::Number id="longitude" name="longitude"
                                        placeholder="174.5432" step="any" required=true bind_value=lng />
                                </div>
                            </div>

                            // [UNCHANGED] Description
                            <div class="grid gap-2">
                                <Label html_for="description">"Description"</Label>
                                <textarea id="description" name="description" rows="4"
                                    placeholder="Describe this spot..." class=TEXTAREA_CLASS />
                            </div>

                            // ══════════════════════════════════════════════════
                            // [CHANGED] Photos 区域
                            // was: URL 文本输入行（photo_rows + For + Input）
                            // now: Dropzone 拖放上传 + 图片预览网格
                            // ══════════════════════════════════════════════════
                            <div class="grid gap-3">
                                <Label>"Photos"</Label>
                                <p class="text-xs text-muted-foreground">
                                    "First image will be used as cover · Supports JPG, PNG, WebP"
                                </p>

                                // ── Dropzone 区域 ──────────────────────────────
                                <div
                                    class=move || format!(
                                        "border-2 border-dashed rounded-lg p-8 text-center \
                                         cursor-pointer transition-colors select-none {}",
                                        if drag_over.get() {
                                            "border-primary bg-primary/5"
                                        } else {
                                            "border-muted-foreground/30 \
                                             hover:border-primary/50 hover:bg-muted/30"
                                        }
                                    )
                                    // 点击 Dropzone → 触发隐藏 file input
                                    on:click=move |_| {
                                        // ⚠️ rust-analyzer inactive（wasm32 build 正常）
                                        #[cfg(target_arch = "wasm32")]
                                        if let Some(input) = file_input_ref.get() {
                                            input.click();
                                        }
                                    }
                                    on:dragover=move |e| {
                                        e.prevent_default();
                                        drag_over.set(true);
                                    }
                                    on:dragleave=move |_| drag_over.set(false)
                                    on:drop=move |e| {
                                        e.prevent_default();
                                        drag_over.set(false);
                                        // web_sys::DragEvent::data_transfer() 仅 WASM 可用
                                        // ⚠️ rust-analyzer inactive（wasm32 build 正常）
                                        #[cfg(target_arch = "wasm32")]
                                        if let Some(dt) = e.data_transfer() {
                                            if let Some(files) = dt.files() {
                                                process_files(files, next_id, photos);
                                            }
                                        }
                                    }
                                >
                                    // 隐藏 file input，点击 Dropzone 时通过 node_ref 触发
                                    <input
                                        type="file"
                                        accept="image/*"
                                        multiple=true
                                        class="hidden"
                                        node_ref=file_input_ref
                                        on:change=move |e| {
                                            // event_target::<HtmlInputElement> 仅 WASM 可用
                                            // ⚠️ rust-analyzer inactive（wasm32 build 正常）
                                            #[cfg(target_arch = "wasm32")]
                                            {
                                                let input: web_sys::HtmlInputElement =
                                                    event_target(&e);
                                                if let Some(files) = input.files() {
                                                    process_files(files, next_id, photos);
                                                }
                                            }
                                        }
                                    />
                                    <div class="flex flex-col items-center gap-2 \
                                                text-muted-foreground pointer-events-none">
                                        <CloudUpload class="size-8 opacity-50" />
                                        <p class="text-sm font-medium">
                                            "Drop images here or click to select"
                                        </p>
                                    </div>
                                </div>

                                // ── 图片预览网格 ───────────────────────────────
                                // 上传后实时渲染，Done 状态显示预览图，Uploading 显示 spinner
                                <Show when=move || !photos.get().is_empty()>
                                    <div class="grid grid-cols-3 gap-3">
                                        <For
                                            each=move || photos.get()
                                            key=|p| p.id
                                            children=move |item| {
                                                let status = item.status;
                                                let local_id = item.id;
                                                // 第一张图片标注 Cover badge
                                                let is_cover = move || {
                                                    photos.get()
                                                        .first()
                                                        .map(|p| p.id) == Some(local_id)
                                                };
                                                view! {
                                                    <div class="relative group aspect-square \
                                                                rounded-lg overflow-hidden \
                                                                border bg-muted">
                                                        {move || match status.get() {

                                                            // 上传中：居中 spinner
                                                            PhotoStatus::Uploading => view! {
                                                                <div class="w-full h-full flex \
                                                                            items-center justify-center">
                                                                    <div class="animate-spin rounded-full \
                                                                                h-6 w-6 border-2 \
                                                                                border-primary \
                                                                                border-t-transparent" />
                                                                </div>
                                                            }.into_any(),

                                                            // 上传成功：预览图 + Cover badge + 删除按钮
                                                            PhotoStatus::Done { url, img_id } => view! {
                                                                <img src=url.clone()
                                                                    class="w-full h-full object-cover" />

                                                                // Cover badge（仅第一张）
                                                                <Show when=is_cover>
                                                                    <span class="absolute top-1 left-1 \
                                                                                 text-xs bg-primary \
                                                                                 text-primary-foreground \
                                                                                 px-1.5 py-0.5 rounded \
                                                                                 font-medium pointer-events-none">
                                                                        "Cover"
                                                                    </span>
                                                                </Show>

                                                                // 删除按钮（hover 显示）
                                                                // [CHANGED] was: gloo_net::http::Request::delete("/admin/api/photos?id=...")
                                                                // now: 调用 delete_photo server fn，无需 gloo-net 和自定义路由
                                                                <button
                                                                    type="button"
                                                                    class="absolute top-1 right-1 \
                                                                           bg-destructive \
                                                                           text-destructive-foreground \
                                                                           rounded p-1 opacity-0 \
                                                                           group-hover:opacity-100 \
                                                                           transition-opacity"
                                                                    on:click=move |e| {
                                                                        e.stop_propagation();
                                                                        let id = img_id.clone();
                                                                        // delete_photo 是普通 server fn，
                                                                        // 无需 #[cfg] 隔离（不涉及 web_sys）
                                                                        leptos::task::spawn_local(async move {
                                                                            use crate::server::spots::delete_photo;
                                                                            // 无论成功失败都移除预览
                                                                            // 孤立文件由后端清理任务兜底
                                                                            let _ = delete_photo(id).await;
                                                                            photos.update(|v| {
                                                                                v.retain(|p| p.id != local_id)
                                                                            });
                                                                        });
                                                                    }
                                                                >
                                                                    <Trash2 class="size-3" />
                                                                </button>
                                                            }.into_any(),

                                                            // 上传失败：错误信息
                                                            PhotoStatus::Failed(err) => view! {
                                                                <div class="w-full h-full flex flex-col \
                                                                            items-center justify-center \
                                                                            gap-1 p-2">
                                                                    <p class="text-xs text-destructive \
                                                                              text-center line-clamp-3">
                                                                        {err}
                                                                    </p>
                                                                </div>
                                                            }.into_any(),
                                                        }}
                                                    </div>
                                                }
                                            }
                                        />
                                    </div>
                                </Show>

                                // ── 隐藏字段：只收集 Done 状态的 URL ──────────
                                // ActionForm 提交时自动携带，与 opening_hours_json 隐藏字段同样机制
                                {move || photos.get().into_iter()
                                    .filter_map(|p| {
                                        if let PhotoStatus::Done { url, .. } = p.status.get() {
                                            Some(view! {
                                                <input type="hidden" name="photo_urls" value=url />
                                            })
                                        } else {
                                            None
                                        }
                                    })
                                    .collect_view()
                                }
                            </div>
                            // ══════════════════════════════════════════════════
                            // end [CHANGED] Photos 区域
                            // ══════════════════════════════════════════════════

                            // [UNCHANGED] Attributes JSON
                            <div class="grid gap-2">
                                <Label html_for="attributes_json">"Attributes (JSON)"</Label>
                                <p class="text-xs text-muted-foreground">
                                    r#"JSON array, e.g. [{"type":"family_friendly","label":"Family Friendly"}]"#
                                </p>
                                <textarea id="attributes_json" name="attributes_json" rows="3"
                                    placeholder=r#"[{"type": "family_friendly", "label": "Family Friendly"}]"#
                                    class=TEXTAREA_CLASS />
                            </div>

                            // [UNCHANGED] Phone & Website
                            <div class="grid grid-cols-2 gap-4">
                                <div class="grid gap-2">
                                    <Label html_for="phone">"Phone (optional)"</Label>
                                    <Input r#type=InputType::Tel id="phone" name="phone"
                                        placeholder="+64 9 123 4567" />
                                </div>
                                <div class="grid gap-2">
                                    <Label html_for="website">"Website (optional)"</Label>
                                    <Input r#type=InputType::Url id="website" name="website"
                                        placeholder="https://example.com" />
                                </div>
                            </div>

                            // [UNCHANGED] Opening Hours
                            <div class="grid gap-3">
                                <Label>"Opening Hours"</Label>
                                <div class="rounded-md border overflow-hidden">
                                    <table class="w-full text-sm">
                                        <thead class="bg-muted text-muted-foreground">
                                            <tr>
                                                <th class="px-3 py-2 text-left font-medium">"Day"</th>
                                                <th class="px-3 py-2 text-left font-medium">"Status"</th>
                                                <th class="px-3 py-2 text-left font-medium">"Open"</th>
                                                <th class="px-3 py-2 text-left font-medium">"Close"</th>
                                            </tr>
                                        </thead>
                                        <tbody>
                                            {schedules.get_value().iter().map(|(day_name, sched)| {
                                                let status = sched.status;
                                                let open_time = sched.open_time;
                                                let close_time = sched.close_time;
                                                let day_name = *day_name;
                                                view! {
                                                    <tr class="border-t">
                                                        <td class="px-3 py-2 font-medium">{day_name}</td>
                                                        <td class="px-3 py-2">
                                                            <select
                                                                class="border rounded px-2 py-1 text-sm bg-background"
                                                                on:change=move |e| {
                                                                    status.set(match event_target_value(&e).as_str() {
                                                                        "closed"  => DayStatus::Closed,
                                                                        "open24h" => DayStatus::Open24h,
                                                                        _         => DayStatus::Open,
                                                                    });
                                                                }
                                                            >
                                                                <option value="open"
                                                                    selected=move || status.get() == DayStatus::Open>
                                                                    "Open"
                                                                </option>
                                                                <option value="closed"
                                                                    selected=move || status.get() == DayStatus::Closed>
                                                                    "Closed"
                                                                </option>
                                                                <option value="open24h"
                                                                    selected=move || status.get() == DayStatus::Open24h>
                                                                    "24 Hours"
                                                                </option>
                                                            </select>
                                                        </td>
                                                        <td class="px-3 py-2">
                                                            <input type="time"
                                                                class="border rounded px-2 py-1 text-sm \
                                                                       bg-background disabled:opacity-40"
                                                                prop:value=move || open_time.get()
                                                                prop:disabled=move || status.get() != DayStatus::Open
                                                                on:input=move |e| open_time.set(event_target_value(&e))
                                                            />
                                                        </td>
                                                        <td class="px-3 py-2">
                                                            <input type="time"
                                                                class="border rounded px-2 py-1 text-sm \
                                                                       bg-background disabled:opacity-40"
                                                                prop:value=move || close_time.get()
                                                                prop:disabled=move || status.get() != DayStatus::Open
                                                                on:input=move |e| close_time.set(event_target_value(&e))
                                                            />
                                                        </td>
                                                    </tr>
                                                }
                                            }).collect_view()}
                                        </tbody>
                                    </table>
                                </div>
                                <input type="hidden" name="opening_hours_json"
                                    prop:value=move || hours_memo.get() />
                            </div>

                            // [UNCHANGED] 服务端错误
                            {move || {
                                create_action.value().get()
                                    .and_then(|r| r.err())
                                    .map(|e| view! {
                                        <p class="text-sm text-destructive">{e.to_string()}</p>
                                    })
                            }}

                            // [UNCHANGED] 操作按钮
                            <div class="flex justify-end gap-3 pt-2">
                                <Button
                                    variant=ButtonVariant::Outline
                                    on:click=move |_| {
                                        navigate_cancel("/spots", NavigateOptions::default());
                                    }
                                >
                                    "Cancel"
                                </Button>
                                <Button attr:disabled=move || create_action.pending().get()>
                                    {move || if create_action.pending().get() {
                                        "Creating..."
                                    } else {
                                        "Create Spot"
                                    }}
                                </Button>
                            </div>

                        </div>
                    </ActionForm>
                </CardContent>
            </Card>
        </div>
    }
}
```

**变更摘要**

| 位置 | 变更 |
|------|------|
| `use icons::` | 新增 `CloudUpload` |
| `PhotoItem` / `PhotoStatus` | 新增（替代 `(u32, RwSignal<String>)` 元组） |
| `upload_file` / `process_files` | 新增，`#[cfg(target_arch = "wasm32")]` 隔离 |
| `next_id` | 初始值从 `1` 改为 `0` |
| `photo_rows` | 删除，替换为 `photos: RwSignal<Vec<PhotoItem>>` |
| `drag_over` | 新增 `RwSignal<bool>` |
| `file_input_ref` | 新增 `NodeRef::<leptos::html::Input>` |
| Photo URLs 视图块 | 删除 URL 文本输入行，替换为 Dropzone + 预览网格 + 隐藏字段 |

---

## 六、新增依赖

### `explonz_admin/Cargo.toml`

不再需要 `gloo-net` 和 `wasm-bindgen-futures`（上传改走 server fn，`spawn_local` 用 `leptos::task`）。

需要两处改动：

**1. 显式添加 `server_fn 0.8` 并开启 `multipart` feature**

> ⚠️ 关键：Leptos 0.8 依赖 `server_fn 0.8.x`（非 0.7.x）。
> 必须指定 `version = "0.8"`，否则 Cargo 会引入两个不兼容版本，导致类型不可用。
> `#[server]` 宏中需用 `input = server_fn::codec::MultipartFormData`（类型路径），
> 不能用旧的字符串形式 `"MultipartFormData"`（Leptos 0.8 的宏已不支持此格式）。

```toml
server_fn = { version = "0.8", features = ["multipart"] }
```

**2. WASM target 补充 `web-sys` Feature**，供 Dropzone 读取文件和 FormData 使用：

```toml
[target.'cfg(all(target_arch = "wasm32", target_os = "unknown"))'.dependencies]
# web-sys 由 Leptos 传递依赖，但 FormData / FileList 等 feature 需显式声明
web-sys = { version = "0.3", features = [
    "FormData",          # 构造 multipart 数据传给 upload_photo server fn
    "File",              # 读取拖入/选择的文件对象
    "FileList",          # input.files() 返回类型
    "HtmlInputElement",  # event_target::<HtmlInputElement> 取 files()
    "DataTransfer",      # DragEvent.data_transfer()
] }
```

### `Cargo.toml`（workspace root）

```toml
# tower-http 已有，新增 fs feature
tower-http = { version = "0.6.6", features = [
    "trace", "timeout", "limit", "cors",
    "normalize-path", "auth",
    "fs"   # 新增，ServeDir 依赖
]}

# reqwest 新增 multipart feature（spots.rs 中 reqwest::multipart::Form 转发图片数据）
reqwest = { version = "0.13.4", features = ["json", "query", "multipart"] }
```

---

## 七、后端定时清理任务

直接扫描本地 `UPLOAD_DIR`，无需 HTTP 调用图片服务器。

### 7.1 `src/service/image_cleanup.rs`

```rust
use chrono::Utc;
use sea_orm::{ConnectionTrait, DatabaseConnection, Statement};
use std::collections::HashSet;
use std::time::Duration;
use tokio::time;

pub async fn run_image_cleanup_task(
    db: DatabaseConnection,
    upload_dir: String,
    interval_secs: u64,
    grace_secs: i64,
) {
    let mut ticker = time::interval(Duration::from_secs(interval_secs));
    ticker.tick().await; // 跳过启动时的立即触发

    loop {
        ticker.tick().await;
        if let Err(e) = cleanup_once(&db, &upload_dir, grace_secs).await {
            tracing::error!("image cleanup error: {e}");
        }
    }
}

async fn cleanup_once(
    db: &DatabaseConnection,
    upload_dir: &str,
    grace_secs: i64,
) -> anyhow::Result<()> {
    // 1. 查询 DB 中所有在用的图片 URL，提取文件名
    let rows = db
        .query_all(Statement::from_string(
            db.get_database_backend(),
            "SELECT DISTINCT unnest(photo_urls) AS url FROM spots".to_string(),
        ))
        .await?;
    let db_filenames: HashSet<String> = rows
        .iter()
        .filter_map(|r| r.try_get::<String>("", "url").ok())
        .filter_map(|url| url.split('/').last().map(String::from))
        .collect();

    // 2. 扫描上传目录
    let cutoff = Utc::now() - chrono::Duration::seconds(grace_secs);
    let mut dir = tokio::fs::read_dir(upload_dir).await?;
    let mut deleted = 0usize;
    let mut checked = 0usize;

    while let Some(entry) = dir.next_entry().await? {
        let filename = entry.file_name().to_string_lossy().to_string();
        checked += 1;

        if db_filenames.contains(&filename) {
            continue; // 仍在使用
        }

        // 检查文件修改时间是否超过宽限期
        let metadata = entry.metadata().await?;
        let modified: chrono::DateTime<Utc> = metadata.modified()?.into();
        if modified >= cutoff {
            continue; // 在宽限期内，跳过
        }

        // 删除孤立文件
        match tokio::fs::remove_file(entry.path()).await {
            Ok(_) => {
                tracing::info!("deleted orphan image: {filename}");
                deleted += 1;
            }
            Err(e) => {
                tracing::warn!("failed to delete {filename}: {e}");
            }
        }
    }

    tracing::info!("image cleanup done: checked={checked}, deleted={deleted}");
    Ok(())
}
```

### 7.2 `src/service/mod.rs`

```rust
pub mod auth;
pub mod email;
pub mod spots;
pub mod image_cleanup; // 新增
```

### 7.3 `src/main.rs` 启动清理任务

```rust
let cleanup_interval = std::env::var("IMAGE_CLEANUP_INTERVAL_SECS")
    .ok().and_then(|s| s.parse().ok()).unwrap_or(3600u64);
let cleanup_grace = std::env::var("IMAGE_CLEANUP_GRACE_SECS")
    .ok().and_then(|s| s.parse().ok()).unwrap_or(1800i64);

tokio::spawn(crate::service::image_cleanup::run_image_cleanup_task(
    db.clone(),
    upload_dir.clone(),
    cleanup_interval,
    cleanup_grace,
));
```

---

## 八、改动文件汇总

| 文件 | 变更类型 | 说明 |
|------|---------|------|
| `src/api/images/handler.rs` | 新增 | 上传/删除图片 Axum handler，写入本地 UPLOAD_DIR |
| `src/api/images/mod.rs` | 新增 | images 路由模块 |
| `src/api/mod.rs` | 修改 | 注册 `/api/images` 路由；添加 `/static/images` ServeDir |
| `src/application.rs` | 修改 | `AppState` 新增 `upload_dir`、`public_url` |
| `src/main.rs` | 修改 | 初始化 upload_dir；启动 cleanup background task |
| `src/service/image_cleanup.rs` | 新增 | 定时扫描目录删除孤立图片 |
| `src/service/mod.rs` | 修改 | 新增 `pub mod image_cleanup` |
| `Cargo.toml`（workspace root） | 修改 | `tower-http` 新增 `fs` feature |
| `explonz_admin/src/server/spots.rs` | 修改 | 新增 `upload_photo`（MultipartFormData）、`delete_photo`、`PhotoUploadResponse` |
| `explonz_admin/src/pages/spots/addition.rs` | 修改 | Dropzone 组件；调用 server fn 替代 gloo-net |
| `explonz_admin/Cargo.toml` | 修改 | WASM target 新增 `web-sys` features（FormData、FileList 等） |
| `explonz_bnd/.env` | 修改 | 新增 `UPLOAD_DIR`、`PUBLIC_URL`、`IMAGE_CLEANUP_INTERVAL_SECS`、`IMAGE_CLEANUP_GRACE_SECS` |

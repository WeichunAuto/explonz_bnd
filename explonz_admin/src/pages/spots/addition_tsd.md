# TDS：创建 Spot 功能

## 一、整体架构

```
浏览器
  │  表单提交
  ▼
explonz_admin（Leptos SSR）
  │  #[server] fn create_spot()
  │  1. 从请求 Cookie 中取出 access_token
  │  2. 解析 attributes_json（photo_urls 直接作为 Vec<String> 接收，无需解析）
  │  3. reqwest POST /api/spots  +  Authorization: Bearer <access_token>
  ▼
explonz_bnd（Axum）
  │  JWT 中间件校验 Bearer token（与 Admin 共用 get_jwt()）
  │  handler → service → SeaORM → PostgreSQL
  ▼
返回 ApiResponse<SpotDto>，Admin 反序列化后响应客户端
```

---

## 二、认证策略

### 共用 JWT 密钥

Admin 登录（`admin_login` server fn）与后端（`explonz_bnd` auth 中间件）均通过
`explonz_shared::common::auth::get_jwt()` 获取同一个 `Jwt` 实例（密钥来自相同的运行时配置）。

因此，Admin 签发的 `access_token` 可直接被后端 auth 中间件识别为合法 token。

### 转发流程

```
Admin #[server] fn
  ├─ extract CookieJar（via leptos_axum::extract）
  ├─ jar.get("access_token") → token: String
  └─ reqwest::Client::new()
       .post("{BACKEND_URL}/api/spots")
       .bearer_auth(&token)          // Authorization: Bearer <token>
       .json(&body)
       .send()
```

后端 auth 中间件（`src/middleware.rs` → `JWTAuth`）：
```
Authorization: Bearer <token>
  └─ get_jwt().decode(token) → Principal { id: "admin_<email>", ... }
  └─ request.extensions_mut().insert(principal)  // 注入 Principal
```

### 环境变量

引入 `dotenvy` 在进程启动时自动加载 `explonz_admin/.env`，所有变量统一在文件中管理，
启动命令不再需要命令行前缀传参。`.env` 已加入 `.gitignore`，不会上传到 GitHub。

**涉及改动：**

**① 工作区 `Cargo.toml`** — 取消 `dotenvy` 注释：
```toml
# 改前
#dotenvy = "0.15.7"

# 改后
dotenvy = "0.15.7"
```

**② `explonz_admin/Cargo.toml`** — 新增 dotenvy 为 SSR 可选依赖：
```toml
[dependencies]
dotenvy = { workspace = true, optional = true }

[features]
ssr = [
    ...
    "dep:dotenvy",
]
```

**③ `explonz_admin/src/main.rs`** — 在 `main()` 开头加载 `.env`：
```rust
#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok(); // 加载 explonz_admin/.env，变量注入进程环境
    // ... 其余不变
}
```

**④ `explonz_admin/.env`** — 统一管理所有运行时变量（新增 `BACKEND_URL`）：
```bash
ADMIN_ACCOUNT=bobby
ADMIN_PASSWORD_HASH=$2b$12$vDPKXDGUTRoqwlPWVLDyf.6nXaJnKFNIbc0WrYBzeTGkWIkTsH4iq
BACKEND_URL=http://192.168.68.50:3005
```

启动命令简化为：
```bash
cargo leptos watch --project explonz_admin
```

| 变量 | 来源 | 说明 |
|------|------|------|
| `ADMIN_ACCOUNT` | `explonz_admin/.env` | 管理员账号（已有） |
| `ADMIN_PASSWORD_HASH` | `explonz_admin/.env` | 管理员密码哈希（已有） |
| `BACKEND_URL` | `explonz_admin/.env` | 后端 Axum 地址，与 `config/dev.yaml` 的 `server.host:port` 对应（新增） |

---

## 三、后端 `explonz_bnd`

### 3.1 新增文件

#### `src/api/spots/dto.rs`

```rust
#[derive(Debug, Deserialize)]
pub struct CreateSpotRequest {
    pub name: String,
    pub location: String,
    pub latitude: f64,
    pub longitude: f64,
    pub description: String,     // 默认 ""
    pub photo_urls: Vec<String>, // 默认 []
    pub attributes: Value,       // 默认 []
}
```

#### `src/api/spots/handler.rs`

```rust
#[debug_handler]
pub async fn create_spot(
    State(AppState { db }): State<AppState>,
    Json(req): Json<CreateSpotRequest>,
) -> ApiResult<SpotDto> {
    let spot = create_spot_service(&db, req).await
        .map_err(|e| ApiError::InternalError(e))?;
    Ok(ApiResponse::success("spot created", Some(spot)))
}
```

handler 不需要显式提取 `Principal`，auth 中间件已在路由层面完成鉴权。

#### `src/api/spots/mod.rs`

```rust
pub(crate) fn routes() -> Router<AppState> {
    Router::new().route("/spots", post(create_spot))
}
```

#### `src/service/spots.rs`

```rust
pub async fn create_spot_service(
    db: &DatabaseConnection,
    req: CreateSpotRequest,
) -> anyhow::Result<SpotDto> {
    let model = spots::ActiveModel {
        name: Set(req.name),
        location: Set(req.location),
        latitude: Set(req.latitude),
        longitude: Set(req.longitude),
        description: Set(req.description),
        photo_urls: Set(req.photo_urls),
        attributes: Set(req.attributes),
        ..Default::default()  // id 由 uuidv7() 数据库生成，rating 默认 0.0
    };
    let result = model.insert(db).await?;
    // crate::entity::spots::Model 与 explonz_shared::entity::spots::Model 为不同类型，
    // 无法使用 SpotDto::from()，手动构造
    Ok(SpotDto {
        id: result.id,
        name: result.name,
        rating: result.rating,
        location: result.location,
        latitude: result.latitude,
        longitude: result.longitude,
        description: result.description,
        photo_urls: result.photo_urls,
        attributes: result.attributes,
        created_at: result.created_at.into(),
        updated_at: result.updated_at.into(),
    })
}
```

### 3.2 修改文件

**`src/service/mod.rs`**
```rust
pub mod spots; // 新增
```

**`src/api/mod.rs`**
```rust
pub(crate) mod spots; // 新增

pub async fn build_routes() -> Router<AppState> {
    Router::new()
        .route("/", get(index))
        .nest("/api", user::routes())
        .nest("/api", spots::routes()) // 新增，置于 route_layer 之前
        .route_layer(get_auth_layer()) // Bearer JWT 校验应用于以上所有 /api 路由
        .nest("/auth", auth::routes())
        ...
}
```

---

## 四、Admin `explonz_admin`

### 4.1 `Cargo.toml`

```toml
[dependencies]
dotenvy    = { workspace = true, optional = true }
reqwest    = { workspace = true, optional = true }
serde_json = { workspace = true, optional = true }

[features]
ssr = [
    ...
    "dep:dotenvy",
    "dep:reqwest",
    "dep:serde_json",
]
```

> `dotenvy` 已在工作区 `Cargo.toml` 中定义（取消注释后可用）。

### 4.2 地理编码（Location → Latitude / Longitude）

#### 方案选型

使用 **OpenStreetMap Nominatim** 免费地理编码 API，无需 API Key，适合管理后台低频使用。

| 项 | 说明 |
|----|------|
| 接口 | `GET https://nominatim.openstreetmap.org/search` |
| 必填 Header | `User-Agent: explonz-admin/1.0`（Nominatim 使用政策要求） |
| 请求参数 | `q=<address>&format=json&limit=1` |
| 响应字段 | `lat: String, lon: String`（字符串形式的浮点数） |
| 无结果 | 返回空数组，转为 `ServerFnError` 提示用户 |
| 新增依赖 | 无（`reqwest` 已在 SSR feature 中） |

#### 交互流程

```
用户输入 location → 点击 "Lookup" 按钮
  └─ 触发 geocode_action.dispatch(location_val)
       └─ server fn geocode_location() → Nominatim API
            ├─ 成功：Effect 将 (lat, lon) 写入 lat / lng RwSignal
            │         lat / lng input 自动填入，用户可手动修改
            └─ 失败：location 字段下方显示错误提示
```

#### `src/server/spots.rs` — 新增 `geocode_location`

```rust
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

    let result = results.into_iter().next()
        .ok_or_else(|| ServerFnError::new("No location found for this address"))?;

    Ok((
        result.lat.parse::<f64>().map_err(|e| ServerFnError::new(e.to_string()))?,
        result.lon.parse::<f64>().map_err(|e| ServerFnError::new(e.to_string()))?,
    ))
}
```

#### UI 层改动（`addition.rs`）

**需要解决的核心问题：Lookup 按钮如何读取 Location 输入框的当前值？**

`ActionForm` 在提交时才会序列化所有字段值，点击 Lookup 按钮不触发 form submit，
因此无法通过表单机制直接拿到 `location` 的当前文字。

解决方式：给 Location 的 `Input` 增加 `bind_value` 双向绑定，
把输入框的实时内容同步到一个 `RwSignal<String>`（设为 `location_val`）。
Lookup 按钮点击时读取该 signal 的值，传给 `geocode_action`。

**状态说明：**

| signal | 作用 |
|--------|------|
| `location_val: RwSignal<String>` | 双向绑定 Location 输入框，供 Lookup 按钮在点击时读取当前地址文字 |
| `lat: RwSignal<String>` | 双向绑定 Latitude 输入框，geocode 成功后自动写入，用户仍可手动修改 |
| `lng: RwSignal<String>` | 双向绑定 Longitude 输入框，同上 |
| `geocode_action` | `Action`，点击 Lookup 时 dispatch，异步调用 `geocode_location` server fn |

**`geocode_action` 触发链路：**

```
用户在 Location 输入框打字
  └─ bind_value 实时同步 → location_val 更新

用户点击 "Lookup" 按钮（attr:type="button"，不触发 form submit）
  └─ on:click 读取 location_val.get_untracked()（当前地址文字）
       └─ geocode_action.dispatch(address)
            └─ 异步调用 geocode_location(address) server fn → Nominatim API
                 ├─ 成功：Effect 监听到结果 → lat.set() / lng.set()
                 │         Latitude / Longitude 输入框自动填入，用户可再手动调整
                 └─ 失败：location 字段下方展示错误提示（小字红色）
```

> `location_val` 看似"只读不用"，实际是 Lookup 按钮点击时的数据来源。
> `bind_value` 还确保 `ActionForm` 提交时 location 字段的值与信号保持一致。

**Location 字段布局变化：**

```
原：[ location input                              ]
新：[ location input (flex-1)  ] [ Lookup 按钮   ]
    <geocode 错误提示（小字红色，仅 geocode 失败时显示）>
```

- Lookup 按钮：idle 显示 `"Lookup"`，pending 显示 `"Looking up..."`，pending 期间 disabled
- Geocode 错误与主表单提交错误相互独立，各自展示

**Latitude / Longitude 字段变化：**

- 原：纯受控输入，用户手动填写
- 新：增加 `bind_value` 绑定，geocode 成功后由 Effect 自动写入，用户仍可手动覆盖

---

### 4.3 `src/server/spots.rs` — 新增 `create_spot`

`photo_urls` 接收 `Vec<String>`，由表单中多个同名 `<input name="photo_urls">` 序列化而来，
无需在 server fn 中手动解析。

```rust
#[server(CreateSpot, "/api")]
pub async fn create_spot(
    name: String,
    location: String,
    latitude: f64,
    longitude: f64,
    description: String,
    photo_urls: Vec<String>,     // 多个同名 input 直接反序列化为 Vec
    attributes_json: String,
) -> Result<SpotDto, ServerFnError> {
    // 1. 取出 access_token cookie 作为 Bearer token
    let jar: CookieJar = extract().await?;
    let token = jar.get("access_token")
        .map(|c| c.value().to_string())
        .ok_or_else(|| ServerFnError::new("Not authenticated"))?;

    // 2. 过滤空值
    let photo_urls: Vec<String> = photo_urls.into_iter()
        .filter(|s| !s.trim().is_empty())
        .collect();

    // 3. 解析 attributes JSON，为空时默认 []
    let attributes: serde_json::Value = if attributes_json.trim().is_empty() {
        serde_json::Value::Array(vec![])
    } else {
        serde_json::from_str(&attributes_json)
            .map_err(|e| ServerFnError::new(format!("Invalid JSON: {e}")))?
    };

    // 4. 构造请求体
    let body = serde_json::json!({
        "name": name, "location": location,
        "latitude": latitude, "longitude": longitude,
        "description": description,
        "photo_urls": photo_urls, "attributes": attributes,
    });

    // 5. 转发请求到后端（携带 Bearer token）
    let backend_url = std::env::var("BACKEND_URL")
        .unwrap_or_else(|_| "http://127.0.0.1:3000".to_string());

    let resp = reqwest::Client::new()
        .post(format!("{backend_url}/api/spots"))
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

    #[derive(Deserialize)]
    struct BackendResponse { data: Option<SpotDto> }

    let parsed: BackendResponse = resp.json().await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    parsed.data.ok_or_else(|| ServerFnError::new("No data returned"))
}
```

### 4.3 `src/pages/spots/addition.rs` — 完整代码

```rust
use icons::Trash2;
use leptos::prelude::*;
use leptos_router::hooks::use_navigate;
use leptos_router::NavigateOptions;

use crate::components::ui::button::{Button, ButtonVariant};
use crate::components::ui::card::{Card, CardContent, CardHeader, CardTitle};
use crate::components::ui::input::{Input, InputType};
use crate::components::ui::label::Label;
use crate::server::spots::{geocode_location, CreateSpot};

const TEXTAREA_CLASS: &str = "text-foreground placeholder:text-muted-foreground border-input \
    flex w-full rounded-md border bg-transparent px-3 py-2 text-sm shadow-xs outline-none \
    focus-visible:border-ring focus-visible:ring-ring/50 focus-visible:ring-2 \
    dark:bg-input/30 resize-none";

#[component]
pub fn SpotAddition() -> impl IntoView {
    let create_action = ServerAction::<CreateSpot>::new();
    let navigate = use_navigate();
    let navigate_cancel = navigate.clone();

    // --- 地理编码 ---
    // location_val：双向绑定 Location 输入框，供 Lookup 按钮点击时读取当前地址
    let location_val = RwSignal::new(String::new());
    // lat / lng：双向绑定经纬度输入框，geocode 成功后自动写入，用户仍可手动覆盖
    let lat = RwSignal::new(String::new());
    let lng = RwSignal::new(String::new());
    // geocode_action：点击 Lookup 时 dispatch，异步调用 geocode_location server fn
    let geocode_action = Action::new(move |address: &String| {
        let address = address.clone();
        async move { geocode_location(address).await }
    });
    // geocode 成功后将结果写入 lat / lng signal，输入框随之自动填入
    Effect::new(move |_| {
        if let Some(Ok((lat_val, lng_val))) = geocode_action.value().get() {
            lat.set(format!("{lat_val}"));
            lng.set(format!("{lng_val}"));
        }
    });

    // --- 动态 Photo URL 行 ---
    // 使用稳定 ID 作为 For key，避免删除时索引偏移导致组件复用错误
    let next_id = RwSignal::new(1u32);
    let photo_rows: RwSignal<Vec<(u32, RwSignal<String>)>> =
        RwSignal::new(vec![(0, RwSignal::new(String::new()))]);

    // 提交成功后跳转列表页
    Effect::new(move |_| {
        if let Some(Ok(_)) = create_action.value().get() {
            navigate("/spots", NavigateOptions::default());
        }
    });

    view! {
        <div class="p-6 max-w-2xl mx-auto">
            <h1 class="text-2xl font-semibold mb-6">"Create Spot"</h1>

            <Card>
                <CardHeader>
                    <CardTitle>"Spot Details"</CardTitle>
                </CardHeader>
                <CardContent>
                    <ActionForm action=create_action>
                        <div class="flex flex-col gap-5">

                            // Name
                            <div class="grid gap-2">
                                <Label html_for="name">"Name"</Label>
                                <Input
                                    id="name"
                                    name="name"
                                    placeholder="e.g. Kumeu Orchard"
                                    required=true
                                />
                            </div>

                            // Location + Lookup 按钮
                            // bind_value=location_val 让输入框实时同步到 signal，
                            // 点击 Lookup 时通过 location_val.get_untracked() 读取当前地址
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
                                        attr:type="button"
                                        variant=ButtonVariant::Outline
                                        attr:disabled=move || geocode_action.pending().get()
                                        on:click=move |_| {
                                            geocode_action.dispatch(
                                                location_val.get_untracked()
                                            );
                                        }
                                    >
                                        {move || if geocode_action.pending().get() {
                                            "Looking up..."
                                        } else {
                                            "Lookup"
                                        }}
                                    </Button>
                                </div>
                                // Geocode 错误独立展示，不影响主表单提交错误区
                                {move || geocode_action.value().get()
                                    .and_then(|r| r.err())
                                    .map(|e| view! {
                                        <p class="text-xs text-destructive">{e.to_string()}</p>
                                    })
                                }
                            </div>

                            // Latitude & Longitude（双列）
                            // bind_value 绑定到 lat / lng signal：
                            //   - geocode 成功时由 Effect 自动写入
                            //   - 用户也可直接在此手动输入或修改
                            <div class="grid grid-cols-2 gap-4">
                                <div class="grid gap-2">
                                    <Label html_for="latitude">"Latitude"</Label>
                                    <Input
                                        r#type=InputType::Number
                                        id="latitude"
                                        name="latitude"
                                        bind_value=lat
                                        placeholder="-36.7896"
                                        step="any"
                                        required=true
                                    />
                                </div>
                                <div class="grid gap-2">
                                    <Label html_for="longitude">"Longitude"</Label>
                                    <Input
                                        r#type=InputType::Number
                                        id="longitude"
                                        name="longitude"
                                        bind_value=lng
                                        placeholder="174.5432"
                                        step="any"
                                        required=true
                                    />
                                </div>
                            </div>

                            // Description
                            <div class="grid gap-2">
                                <Label html_for="description">"Description"</Label>
                                <textarea
                                    id="description"
                                    name="description"
                                    rows="4"
                                    placeholder="Describe this spot..."
                                    class=TEXTAREA_CLASS
                                />
                            </div>

                            // Photo URLs（动态增删行）
                            <div class="grid gap-2">
                                <Label>"Photo URLs"</Label>
                                <p class="text-xs text-muted-foreground">
                                    "First URL will be used as the cover image."
                                </p>
                                <div class="flex flex-col gap-2">
                                    // 每行渲染一个 name="photo_urls" 的 input
                                    // ActionForm 将同名重复 input 序列化为 Vec<String>
                                    <For
                                        each=move || photo_rows.get()
                                        key=|(id, _)| *id
                                        children=move |(id, url_signal)| {
                                            view! {
                                                <div class="flex gap-2 items-center">
                                                    <Input
                                                        name="photo_urls"
                                                        bind_value=url_signal
                                                        placeholder="https://example.com/photo.jpg"
                                                    />
                                                    <Button
                                                        attr:type="button"
                                                        variant=ButtonVariant::Ghost
                                                        attr:disabled=move || photo_rows.get().len() <= 1
                                                        on:click=move |_| {
                                                            photo_rows.update(|v| {
                                                                v.retain(|(row_id, _)| *row_id != id)
                                                            });
                                                        }
                                                    >
                                                        <Trash2 class="size-4" />
                                                    </Button>
                                                </div>
                                            }
                                        }
                                    />
                                    <Button
                                        attr:type="button"
                                        variant=ButtonVariant::Outline
                                        class="self-start"
                                        on:click=move |_| {
                                            let id = next_id.get_untracked();
                                            next_id.update(|n| *n += 1);
                                            photo_rows.update(|v| {
                                                v.push((id, RwSignal::new(String::new())))
                                            });
                                        }
                                    >
                                        "+ Add Photo"
                                    </Button>
                                </div>
                            </div>

                            // Attributes JSON
                            <div class="grid gap-2">
                                <Label html_for="attributes_json">"Attributes (JSON)"</Label>
                                <p class="text-xs text-muted-foreground">
                                    r#"JSON array, e.g. [{"type":"family_friendly","label":"Family Friendly"}]"#
                                </p>
                                <textarea
                                    id="attributes_json"
                                    name="attributes_json"
                                    rows="3"
                                    placeholder=r#"[{"type": "family_friendly", "label": "Family Friendly"}]"#
                                    class=TEXTAREA_CLASS
                                />
                            </div>

                            // 主表单提交错误内联展示
                            {move || {
                                create_action.value().get()
                                    .and_then(|r| r.err())
                                    .map(|e| view! {
                                        <p class="text-sm text-destructive">{e.to_string()}</p>
                                    })
                            }}

                            // 操作按钮
                            <div class="flex justify-end gap-3 pt-2">
                                <Button
                                    variant=ButtonVariant::Outline
                                    attr:type="button"
                                    on:click=move |_| {
                                        navigate_cancel("/spots", NavigateOptions::default());
                                    }
                                >
                                    "Cancel"
                                </Button>
                                <Button
                                    attr:type="submit"
                                    attr:disabled=move || create_action.pending().get()
                                >
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

**序列化原理：**

`ActionForm` 将同名 `<input name="photo_urls">` 序列化为 URL-encoded 重复键：
```
photo_urls=https://a.jpg&photo_urls=https://b.jpg
```
Leptos server fn 通过 `serde` 将其反序列化为 `Vec<String>`，无需手动拼接/解析。

### 4.4 `src/app.rs`

```rust
use crate::pages::spots::{addition::SpotAddition, list::SpotList};

<Route path=path!("/spots")     view=SpotList/>
<Route path=path!("/spots/new") view=SpotAddition/>
```

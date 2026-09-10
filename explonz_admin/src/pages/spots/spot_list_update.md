# Spot List 页面技术方案（含修改 & 删除）

## 一、功能范围

| 功能 | 说明 |
|------|------|
| Spot 列表展示 | 表格展示，显示封面图 / Name / Location / Rating / Actions |
| 删除 Spot | 行内二次确认（与 LabelList 一致），调后端 DELETE |
| 编辑 Spot | 跳转独立编辑路由 `/spots/edit/:spot_id`，字段与创建页一致 |

---

## 二、现有代码问题（需同步修复）

`explonz_admin/src/server/spots.rs` 中的 `GetSpots` server fn 直接操作 SeaORM 查询数据库：

```rust
// 当前实现（违反 Admin Server Function 规则）
#[server(GetSpots, "/api")]
pub async fn get_spots(page: u64, page_size: u64) -> Result<Vec<SpotDto>, ServerFnError> {
    let db = use_context::<DatabaseConnection>()...;
    spots::Entity::find().all(&db).await  // ← 直接查 DB，绕过后端 API
}
```

按 CLAUDE.md 的 Admin Server Function 规则，Server Function 必须通过 `reqwest` 调用 Backend REST API。
**本次实现需一并修复此问题**：后端补充 `GET /api/spots` 端点，server fn 改为 reqwest 调用。

---

## 三、DTO 变更（`explonz_shared/src/common/dto.rs`）

新增 `OpeningHourDto` 和 `SpotDetailDto`。这两个结构同时在 Backend（构造响应）和 Admin（server fn 返回值、组件消费）使用，按 DTO 规则需放入 `explonz_shared`。

在添加前先确认 `explonz_shared/src/common/dto.rs` 中不存在同名结构。

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpeningHourDto {
    pub day_of_week: i16,
    pub is_closed: bool,
    pub is_open_24h: bool,
    pub open_time: Option<String>,   // "HH:MM"，None 表示 is_closed 或 is_open_24h
    pub close_time: Option<String>,  // "HH:MM"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpotDetailDto {
    #[serde(flatten)]
    pub spot: SpotDto,
    pub label_ids: Vec<String>,
    pub opening_hours: Vec<OpeningHourDto>,
}
```

---

## 四、后端变更（`explonz_bnd`）

### 4.1 新增 4 个 API

#### GET `/api/spots` — 获取 Spot 列表（修复 GetSpots server fn 的依赖）

```
GET /api/spots?page=1&page_size=20
→ 返回 ApiResponse<Vec<SpotDto>>
```

Service：`get_spots_service(db, page, page_size) -> anyhow::Result<Vec<SpotDto>>`
- 使用 SeaORM `Paginator` 分页查询 `spots` 表

#### GET `/api/spots/:spot_id` — 获取单个 Spot 详情

```
GET /api/spots/:spot_id
→ 返回 ApiResponse<SpotDetailDto>
```

Service：`get_spot_service(db, spot_id: Uuid) -> anyhow::Result<SpotDetailDto>`
- 查询 `spots` 表获取基础字段
- 查询 `spot_label_assignments` 获取 `label_ids`
- 查询 `spot_opening_hours` 获取营业时间，`NaiveTime` 格式化为 `"HH:MM"` 字符串

#### PUT `/api/spots/:spot_id` — 更新 Spot

```
PUT /api/spots/:spot_id  Body: CreateSpotRequest（复用，不新建 DTO）
→ 返回 ApiResponse<SpotDto>
```

Service：`update_spot_service(db, spot_id: Uuid, req: CreateSpotRequest) -> anyhow::Result<SpotDto>`
- 事务内执行：
  1. `UPDATE spots SET ... WHERE id = spot_id`
  2. `DELETE FROM spot_label_assignments WHERE spot_id = ?`，再批量 INSERT
  3. `DELETE FROM spot_opening_hours WHERE spot_id = ?`，再批量 INSERT
     （先删再插避免 `UNIQUE(spot_id, day_of_week)` 冲突）

#### DELETE `/api/spots/:spot_id` — 删除 Spot

```
DELETE /api/spots/:spot_id
→ 返回 ApiResponse<()>
```

Service：`delete_spot_service(db, spot_id: Uuid) -> anyhow::Result<()>`
- `DELETE FROM spots WHERE id = ?`
- 关联表 `spot_label_assignments`、`spot_opening_hours` 因 `ON DELETE CASCADE` 自动清理

### 4.2 Handler 规范

Handler 只做参数提取、调用 Service、包装响应，不含业务逻辑：

```rust
pub async fn get_spot(
    State(AppState { db, .. }): State<AppState>,
    Path(spot_id): Path<Uuid>,
) -> ApiResult<SpotDetailDto> {
    let detail = get_spot_service(&db, spot_id)
        .await
        .map_err(ApiError::InternalError)?;
    Ok(ApiResponse::success("ok", Some(detail)))
}
```

其余三个 handler（`get_spots`、`update_spot`、`delete_spot`）结构相同。

### 4.3 路由注册（`src/api/spots/mod.rs`）

```rust
pub(crate) fn routes() -> Router<AppState> {
    Router::new()
        .route("/spots",              get(get_spots))      // 新增
        .route("/spots/new",          post(create_spot))
        .route("/spots/images",       post(upload_image))
        .route("/spots/:spot_id",     get(get_spot))       // 新增
        .route("/spots/:spot_id",     put(update_spot))    // 新增
        .route("/spots/:spot_id",     delete(delete_spot)) // 新增
}
```

---

## 五、Admin Server Function 变更（`explonz_admin/src/server/spots.rs`）

所有 server fn 遵循：`extract_token()` → `backend_url()` → `reqwest` → 解析 `ApiResp<T>`。

### 5.1 修复现有 `GetSpots`

```rust
#[server(GetSpots, "/api")]
pub async fn get_spots(page: u64, page_size: u64) -> Result<Vec<SpotDto>, ServerFnError> {
    let token = crate::server::extract_token().await?;
    let backend_url = crate::server::backend_url();

    let resp = reqwest::Client::new()
        .get(format!("{backend_url}/api/spots"))
        .query(&[("page", page), ("page_size", page_size)])
        .bearer_auth(&token)
        .send()
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    if !resp.status().is_success() {
        let msg = resp.text().await.unwrap_or_default();
        return Err(ServerFnError::new(format!("Backend error: {msg}")));
    }

    let parsed: ApiResp<Vec<SpotDto>> = resp.json().await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    parsed.data.ok_or_else(|| ServerFnError::new("No data returned"))
}
```

### 5.2 新增 `GetSpot`

```rust
#[server(GetSpot, "/api")]
pub async fn get_spot(spot_id: String) -> Result<SpotDetailDto, ServerFnError> {
    // reqwest → GET /api/spots/{spot_id}
    // 解析 ApiResp<SpotDetailDto>
}
```

### 5.3 新增 `UpdateSpot`

```rust
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
    // 逻辑与 CreateSpot 完全一致，仅请求改为 PUT /api/spots/{spot_id}
}
```

### 5.4 新增 `DeleteSpot`

```rust
#[server(DeleteSpot, "/api")]
pub async fn delete_spot(spot_id: String) -> Result<(), ServerFnError> {
    // reqwest → DELETE /api/spots/{spot_id}
}
```

---

## 六、Admin 前端变更

### 6.1 新路由（`sidenav_routes_simplified.rs`）

```rust
<ParentRoute path=StaticSegment(ExplonzRoutes::Spots.as_ref()) view=|| view! { <Outlet /> }>
    <Route path=StaticSegment("") view=|| () />
    <Route path=path!("/addition")       view=SpotAddition/>
    <Route path=path!("/spot_list")      view=SpotList/>
    <Route path=path!("/edit/:spot_id")  view=SpotEdit/>    // 新增
    <Route path=path!("/label_list")     view=LabelList/>
</ParentRoute>
```

同时在 `pages/spots/mod.rs` 添加 `pub mod edit;`。

### 6.2 SpotList 页面（`list.rs`）

**参考**：`pages/labels/list.rs` 的整体结构（Resource + Suspense + table + 行内二次确认），直接复用其模式。

**数据加载**：

```rust
let fetch_trigger = RwSignal::new(false);
Effect::new(move |_| { fetch_trigger.set(true); });

let spots = Resource::new(
    move || fetch_trigger.get(),
    move |ready| async move {
        if !ready { return Ok(vec![]); }
        get_spots(1, 20).await
    },
);
```

**状态信号**：

```rust
let deleting_id: RwSignal<Option<String>> = RwSignal::new(None);
let delete_action = ServerAction::<DeleteSpot>::new();
let location = leptos_router::hooks::use_location();
let navigate = leptos_router::hooks::use_navigate();
```

**删除成功后刷新列表**：

```rust
Effect::new(move |_| {
    if matches!(delete_action.value().get(), Some(Ok(_))) {
        spots.refetch();
        deleting_id.set(None);
    }
});
```

**表格列**：

| 列 | 内容 |
|----|------|
| 封面图 | `photo_urls.first()`，无图显示占位符 `<div class="bg-muted ...">` |
| Name | `spot.name` |
| Location | `spot.location`（`truncate` 样式） |
| Rating | `spot.rating`（保留一位小数） |
| Updated | `spot.updated_at`（日期部分） |
| Actions | Edit 按钮 + Delete 二次确认 |

**Edit 按钮点击**（导航到同级 `edit/{id}` 路径）：

```rust
on:click = move |_| {
    let current = location.pathname.get_untracked();
    let base = current.rsplit_once('/').map(|(b, _)| b).unwrap_or("/");
    navigate(&format!("{}/edit/{}", base, spot_id), NavigateOptions::default());
}
```

**Delete 交互**：与 `LabelList` 完全一致——默认显示垃圾桶图标，点击展开 Delete/X 按钮组，确认后 dispatch `DeleteSpot`。

### 6.3 SpotEdit 页面（`edit.rs`）

**路由参数**：

```rust
let params = leptos_router::hooks::use_params_map();
let spot_id = move || params.with(|p| p.get("spot_id").unwrap_or_default());
```

**数据加载**（无需 `fetch_trigger`，路由参数驱动，SSR 和 CSR 均可直接加载）：

```rust
let spot_detail = Resource::new(
    move || spot_id(),
    move |id| async move { get_spot(id).await },
);
let all_labels = Resource::new(|| (), |_| async move { get_labels().await });
```

**表单信号**（结构与 `addition.rs` 保持一致，供数据回填）：

```rust
let form_name: RwSignal<String>        = RwSignal::new(String::new());
let form_location: RwSignal<String>    = RwSignal::new(String::new());
let form_lat: RwSignal<f64>            = RwSignal::new(0.0);
let form_lng: RwSignal<f64>            = RwSignal::new(0.0);
let form_desc: RwSignal<String>        = RwSignal::new(String::new());
let form_phone: RwSignal<String>       = RwSignal::new(String::new());
let form_website: RwSignal<String>     = RwSignal::new(String::new());
let selected_labels: RwSignal<Vec<String>>         = RwSignal::new(vec![]);
let photo_states: RwSignal<Vec<PhotoStatus>>       = RwSignal::new(vec![]);
let opening_hours: RwSignal<[OpeningHourForm; 7]>  = RwSignal::new(...默认值...);
```

**数据回填**：

```rust
Effect::new(move |_| {
    let Some(Ok(detail)) = spot_detail.get() else { return };
    form_name.set(detail.spot.name.clone());
    form_location.set(detail.spot.location.clone());
    form_lat.set(detail.spot.latitude);
    form_lng.set(detail.spot.longitude);
    form_desc.set(detail.spot.description.clone());
    form_phone.set(detail.spot.phone.unwrap_or_default());
    form_website.set(detail.spot.website.unwrap_or_default());
    selected_labels.set(detail.label_ids.clone());
    // photo_urls → PhotoStatus::Done { id: url_basename, url }
    photo_states.set(
        detail.spot.photo_urls.iter()
            .map(|url| PhotoStatus::Done { id: url_basename(url), url: url.clone() })
            .collect()
    );
    // opening_hours 回填（按 day_of_week 索引写入数组）
});
```

**提交**：调用 `UpdateSpot` server fn，参数序列化方式与 `CreateSpot` 完全一致（`photo_urls[]`、`label_ids[]` 括号格式、`opening_hours_json` 序列化为 JSON）。

**成功后导航**（从 `.../spots/edit/{id}` 退两级至 `.../spots`，再拼 `spot_list`）：

```rust
Effect::new(move |_| {
    if matches!(update_action.value().get(), Some(Ok(_))) {
        let current = location.pathname.get_untracked();
        let target = current
            .rsplit_once('/')                              // 去掉 :spot_id → ".../spots/edit"
            .and_then(|(p, _)| p.rsplit_once('/'))        // 去掉 "edit" → ".../spots"
            .map(|(base, _)| format!("{}/spot_list", base))
            .unwrap_or_else(|| "/".to_string());
        navigate(&target, NavigateOptions::default());
    }
});
```

**UI 结构**：与 `addition.rs` 相同（图片上传区、Labels 多选、营业时间表格），仅页头改为 "Edit Spot"，提交按钮改为 "Save Changes"，并去掉 geocode 自动填充经纬度（编辑时经纬度直接回填）。

---

## 七、数据流总览

```
[SpotList 页面]
  WASM
    ↓ Resource(get_spots)
  Server Fn (SSR)
    ↓ reqwest GET /api/spots?page=1&page_size=20
  Backend Handler → get_spots_service → DB
    ↑ ApiResponse<Vec<SpotDto>>

  WASM
    ↓ ServerAction(DeleteSpot { spot_id })
  Server Fn (SSR)
    ↓ reqwest DELETE /api/spots/:id
  Backend Handler → delete_spot_service → DB（CASCADE 清理关联表）

  navigate → [SpotEdit 页面 /spots/edit/:id]

[SpotEdit 页面]
  WASM
    ↓ Resource(get_spot(id))
  Server Fn (SSR)
    ↓ reqwest GET /api/spots/:id
  Backend Handler → get_spot_service → DB（spots + label_assignments + opening_hours）
    ↑ ApiResponse<SpotDetailDto>

  WASM（回填表单后提交）
    ↓ ServerAction(UpdateSpot { spot_id, ... })
  Server Fn (SSR)
    ↓ reqwest PUT /api/spots/:id  Body: CreateSpotRequest
  Backend Handler → update_spot_service（事务：UPDATE + DELETE/re-INSERT 关联表）
    ↑ ApiResponse<SpotDto>

  navigate → /spots/spot_list
```

---

## 八、实现顺序

**Step 1 — 共享 DTO**
- [ ] `explonz_shared/src/common/dto.rs`：新增 `OpeningHourDto`、`SpotDetailDto`

**Step 2 — 后端**
- [ ] `src/service/spots.rs`：新增 `get_spots_service`、`get_spot_service`、`update_spot_service`、`delete_spot_service`
- [ ] `src/api/spots/handler.rs`：新增 `get_spots`、`get_spot`、`update_spot`、`delete_spot`
- [ ] `src/api/spots/mod.rs`：注册 4 条新路由
- [ ] 验证：`cargo check`

**Step 3 — Admin Server Fn**
- [ ] `server/spots.rs`：修复 `GetSpots`（改为 reqwest），新增 `GetSpot`、`UpdateSpot`、`DeleteSpot`

**Step 4 — Admin 前端**
- [ ] `pages/spots/list.rs`：实现 SpotList
- [ ] `pages/spots/edit.rs`：新建 SpotEdit
- [ ] `sidenav_routes_simplified.rs`：注册 `/edit/:spot_id` 路由
- [ ] `pages/spots/mod.rs`：`pub mod edit;`
- [ ] 验证：`cargo leptos build --package explonz_admin`

---

## 九、关键注意事项

- **`UpdateSpotRequest` 复用 `CreateSpotRequest`**：两者结构完全一致，不新建 DTO，直接在 handler 复用 `Json<CreateSpotRequest>`
- **`opening_hours` 更新策略**：DELETE WHERE spot_id=? 后再批量 INSERT，避免 `UNIQUE(spot_id, day_of_week)` 冲突
- **photo_urls 回填**：已有图片构造为 `PhotoStatus::Done`，`id` 字段取 URL 末尾文件名（与 upload 返回的 id 格式一致，供删除使用）
- **`get_spots_service` 的分页参数**：SeaORM 分页从第 0 页开始，前端传入 `page=1` 时需转换为 `paginator.fetch_page(page - 1)`
- **SpotEdit SSR 加载**：编辑页由路由参数驱动加载，不需要 `fetch_trigger` 信号；但回填 Effect 在 SSR 侧不执行（SSR 端没有 DOM），这是正常现象——SSR 只渲染 loading 占位，客户端 hydration 后 Effect 触发回填

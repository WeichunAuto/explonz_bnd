# Actual Seasonal Picks Confirm — Technical Specification Document

## 一、功能背景

`seasonal_pickings` 表存储的是每个地点的"季节性采摘"通用窗口（`season_start_month/day` ~ `season_end_month/day`），
该窗口每年循环复用，不绑定具体年份。

本功能在此基础上新增"当年实际开放日期"的确认机制：
- 管理员可在每年季节开始前/中，为每一条 `seasonal_picking` 记录确认该年实际的开放起止日期。
- 已确认的实际日期供 App 端展示"本赛季开放时间"（而非仅展示通用估算窗口）。
- 未确认时 App 端仍降级展示通用窗口，提示"Dates to be confirmed"。

---

## 二、数据库设计

### 2.1 新增表：`seasonal_picking_actuals`

```sql
CREATE TABLE seasonal_picking_actuals (
    id           UUID        PRIMARY KEY DEFAULT uuidv7(),
    picking_id   UUID        NOT NULL REFERENCES seasonal_pickings(id) ON DELETE CASCADE,
    year         SMALLINT    NOT NULL,               -- 对应年份，如 2026
    actual_start DATE        NOT NULL,               -- 实际开放起始日，如 2026-11-08
    actual_end   DATE        NOT NULL,               -- 实际开放截止日，如 2027-02-16
    notes        TEXT        NOT NULL DEFAULT '',    -- 管理员备注，如 "Confirmed by farm owner"
    created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT uq_seasonal_picking_actuals_picking_year UNIQUE (picking_id, year),
    CONSTRAINT chk_actual_dates      CHECK (actual_end >= actual_start),
    CONSTRAINT chk_actual_start_year CHECK (EXTRACT(YEAR FROM actual_start) = year),
    CONSTRAINT chk_actual_end_year   CHECK (
        EXTRACT(YEAR FROM actual_end) = year
        OR EXTRACT(YEAR FROM actual_end) = year + 1
    )
);

CREATE TRIGGER trg_seasonal_picking_actuals_updated_at
    BEFORE UPDATE ON seasonal_picking_actuals
    FOR EACH ROW EXECUTE FUNCTION fn_set_updated_at();

CREATE INDEX idx_seasonal_picking_actuals_picking_id
    ON seasonal_picking_actuals (picking_id);

CREATE INDEX idx_seasonal_picking_actuals_year
    ON seasonal_picking_actuals (year);
```

**设计说明：**
- `actual_start` / `actual_end` 使用 `DATE` 类型而非 month/day 整数，因为实际日期包含年份信息。
- `UNIQUE(picking_id, year)` 确保每条 picking 每年只有一条确认记录（upsert 语义）。
- `chk_actual_dates`：`actual_end >= actual_start`，确保结束日不早于开始日。
- `chk_actual_start_year`：`EXTRACT(YEAR FROM actual_start) = year`，开始日期必须在 `year` 年内。
- `chk_actual_end_year`：`EXTRACT(YEAR FROM actual_end) IN (year, year + 1)`，
  结束日期允许是当年或下一年，支持跨年季节（如南半球草莓季 2026-11-08 ~ 2027-02-16）。
- `ON DELETE CASCADE`：picking 删除时，对应的实际日期记录也自动清除。

### 2.2 SeaORM Entity

需在 `explonz_shared/src/entity/` 和 `explonz_bnd/src/entity/` 中同步创建
`seasonal_picking_actuals.rs` 并在 `prelude.rs` 中导出（由开发者在 Migration 执行后生成）：

```rust
// explonz_shared/src/entity/seasonal_picking_actuals.rs（预期结构）
pub struct Model {
    pub id: Uuid,
    pub picking_id: Uuid,
    pub year: i16,
    pub actual_start: Date,   // chrono::NaiveDate
    pub actual_end: Date,
    pub notes: String,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}
```

---

## 三、DTO 设计

在 `explonz_shared/src/common/dto.rs` 新增：

```rust
/// 某条 seasonal_picking 在特定年份的实际开放日期
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActualSeasonalPickDto {
    pub id: String,                // UUID string
    pub picking_id: String,        // 关联的 seasonal_pickings.id
    pub year: i16,
    pub actual_start: String,      // "YYYY-MM-DD"
    pub actual_end: String,        // "YYYY-MM-DD"，可跨年（如 "2027-02-16"）
    pub notes: String,
}
```

`SeasonalPickingSpotItemDto`（已有）本次不修改结构。
实际日期通过独立 API 按年查询，在 Admin UI 侧合并，避免污染通用列表接口。

---

## 四、Backend API

### 4.1 路由注册（`src/api/spots/mod.rs`）

新增以下路由，放在 `.route_layer(get_auth_layer())` 之前（需鉴权）：

```
POST   /api/spots/seasonal_pickings/{picking_id}/actuals/new    → upsert_actual_seasonal_pick
GET    /api/spots/seasonal_pickings/actuals/by_year             → get_actuals_by_year  (?year=2026)
DELETE /api/spots/seasonal_pickings/actuals/{actual_id}         → delete_actual_seasonal_pick
```

URL 说明：
- `upsert`（POST new）：同一 picking_id + year 已存在则更新，否则创建。使用 POST /new 而非 PUT，
  因为 upsert 对调用方透明，不需要先知道 actual_id。
- `get_actuals_by_year`：返回指定年份所有已确认的实际日期（不区分 type），Admin 列表页用于显示确认状态。
- `delete`：按 actual_id 删除，允许管理员撤销某年的确认。

### 4.2 Request DTO（`src/api/spots/dto.rs`）

```rust
#[derive(Debug, Deserialize)]
pub struct UpsertActualSeasonalPickRequest {
    pub year: i16,
    pub actual_start: String,   // "YYYY-MM-DD"
    pub actual_end: String,     // "YYYY-MM-DD"
    pub notes: Option<String>,
}
```

### 4.3 Handler（`src/api/spots/handler.rs`）

```rust
// upsert 实际日期
pub async fn upsert_actual_seasonal_pick(
    State(AppState { db, .. }): State<AppState>,
    BPath(picking_id): BPath<Uuid>,
    Json(req): Json<UpsertActualSeasonalPickRequest>,
) -> ApiResult<ActualSeasonalPickDto>

// 按年份查询全部实际日期
#[derive(Deserialize)]
pub struct ActualsByYearQuery { pub year: i16 }

pub async fn get_actuals_by_year(
    State(AppState { db, .. }): State<AppState>,
    Query(q): Query<ActualsByYearQuery>,
) -> ApiResult<Vec<ActualSeasonalPickDto>>

// 删除一条实际日期
pub async fn delete_actual_seasonal_pick(
    State(AppState { db, .. }): State<AppState>,
    BPath(actual_id): BPath<Uuid>,
) -> ApiResult<()>
```

### 4.4 Service（`src/service/spots.rs`）

```rust
// Upsert：ON CONFLICT (picking_id, year) DO UPDATE
// SeaORM 暂不原生支持 on_conflict upsert for composite unique，需用 sea_orm::sea_query 扩展
// 替代方案：先 find_by picking_id + year，有则 update，无则 insert（两步事务）
pub async fn upsert_actual_seasonal_pick_service(
    db: &DatabaseConnection,
    picking_id: Uuid,
    req: UpsertActualSeasonalPickRequest,
) -> anyhow::Result<ActualSeasonalPickDto>

pub async fn get_actuals_by_year_service(
    db: &DatabaseConnection,
    year: i16,
) -> anyhow::Result<Vec<ActualSeasonalPickDto>>

pub async fn delete_actual_seasonal_pick_service(
    db: &DatabaseConnection,
    actual_id: Uuid,
) -> anyhow::Result<()>
```

**upsert 逻辑（两步查找+写入）：**
```
1. SELECT * FROM seasonal_picking_actuals WHERE picking_id = ? AND year = ?
2a. 若找到 → UPDATE actual_start, actual_end, notes
2b. 若未找到 → INSERT 新记录
3. 返回最终记录映射为 ActualSeasonalPickDto
```

---

## 五、Admin Server Functions

在 `explonz_admin/src/server/spots.rs` 新增三个 server fn：

```rust
// Upsert 某条 picking 在指定年份的实际日期
#[server(UpsertActualSeasonalPick, "/api")]
pub async fn upsert_actual_seasonal_pick(
    picking_id: String,
    year: i16,
    actual_start: String,   // "YYYY-MM-DD"
    actual_end: String,     // "YYYY-MM-DD"
    notes: String,
) -> Result<ActualSeasonalPickDto, ServerFnError>
// → POST {backend}/api/spots/seasonal_pickings/{picking_id}/actuals/new

// 获取指定年份全部已确认的实际日期（flat list，keyed by picking_id）
#[server(GetActualsByYear, "/api")]
pub async fn get_actuals_by_year(
    year: i16,
) -> Result<Vec<ActualSeasonalPickDto>, ServerFnError>
// → GET {backend}/api/spots/seasonal_pickings/actuals/by_year?year={year}

// 删除一条实际日期记录
#[server(DeleteActualSeasonalPick, "/api")]
pub async fn delete_actual_seasonal_pick(
    actual_id: String,
) -> Result<(), ServerFnError>
// → DELETE {backend}/api/spots/seasonal_pickings/actuals/{actual_id}
```

---

## 六、Admin UI

### 6.1 整体交互流程

```
SeasonalPicksList
  ├─ 顶部工具栏：年份选择器（默认当前年 2026）
  │      └─ 切换年份 → actuals_resource 重新加载
  │
  └─ Accordion（按 type 分组）
       └─ AccordionContent（每个 type 展开的 spot 表格）
            └─ 每行 spot
                 ├─ [已确认] → 显示绿色徽章"Nov 8 – Feb 16"+ "Edit"按钮
                 ├─ [未确认] → 显示灰色徽章"Not confirmed"+ "Confirm"按钮
                 └─ 点击 Confirm/Edit → 弹出 ActualPickConfirmDialog
                      └─ 填写/修改实际日期 → 保存 → 刷新 actuals_resource
```

### 6.2 `SeasonalPicksList` 页面改造

**新增 Signal 和 Resource：**

```rust
// 当前选中年份
let selected_year = RwSignal::new(chrono::Local::now().year() as i16);

// 加载指定年份的所有 actuals（flat list，picking_id → ActualSeasonalPickDto）
let actuals_resource = Resource::new(
    move || selected_year.get(),
    |year| get_actuals_by_year(year),
);

// actuals HashMap：picking_id -> ActualSeasonalPickDto（在 Suspense view 内构建）
// 用于 O(1) 查找每个 spot 是否已有当年实际日期
```

**年份选择器 UI（放在 Header 行右侧）：**

```rust
// 年份范围：当前年 ± 2（如 2024, 2025, 2026, 2027, 2028）
// 使用 <select> 原生元素即可，无需引入新组件
<select
    class="text-xs border rounded px-2 py-1 bg-background"
    on:change=move |e| {
        let val = event_target_value(&e).parse::<i16>().unwrap_or(2026);
        selected_year.set(val);
    }
>
    {YEAR_RANGE.iter().map(|&y| view! {
        <option value=y.to_string() selected=move || selected_year.get() == y>
            {y.to_string()}
        </option>
    }).collect_view()}
</select>
```

**spot 表格新增"Actual Dates"列：**

原列：Spot | Location | Season Picks | Rating | (View按钮)
新列：Spot | Location | Season Picks | **Actual Dates** | Rating | (操作按钮)

"Actual Dates"列内容：
- 已确认：`<span class="...text-green-600">Nov 8 – Feb 16 ✓</span>`
- 未确认：`<span class="...text-muted-foreground">—</span>`

操作按钮列：
- 已确认：`Edit` 按钮（ButtonVariant::Ghost）
- 未确认：`Confirm` 按钮（ButtonVariant::Outline，带 primary 色调）

**Suspense 读取策略（遵循项目规则）：**

两个 Resource（`picks_resource` + `actuals_resource`）都只在各自的 `<Suspense>` view 闭包内读取，
不在 Effect 中读取，避免 pending 传播到 AuthGuard。

实现方式：外层 `<Suspense>` 处理 `picks_resource`，内层将 `actuals_resource` 在第二个
嵌套 `<Suspense>` 内读取（或合并为单一 `<Suspense>` 待两者就绪）。

推荐合并写法：

```rust
<Suspense fallback=|| view! { <div class="...">"Loading..."</div> }>
    {move || {
        // 同时读取两个 resource
        let picks = picks_resource.get();
        let actuals = actuals_resource.get();

        // 其中一个 pending 时整体 fallback（本地 Suspense 拦截，不冒泡）
        let Some(Ok(picks)) = picks else { return view! {<div/>}.into_any() };
        // actuals 若尚未加载则先以空 map 渲染（宽容处理）
        let actuals_map: HashMap<String, ActualSeasonalPickDto> =
            actuals.and_then(|r| r.ok()).unwrap_or_default()
                .into_iter().map(|a| (a.picking_id.clone(), a)).collect();

        // ... 渲染表格，通过 actuals_map.get(&spot.picking_id) 判断是否已确认
    }}
</Suspense>
```

### 6.3 新组件：`ActualPickConfirmDialog`

文件路径：`explonz_admin/src/pages/spots/actual_pick_confirm_dialog.rs`
（页面级组件，不放入 `components/ui/`，因为与业务逻辑强耦合）

```
Props:
  open:        RwSignal<bool>           // 控制 dialog 显示
  picking_id:  String
  spot_name:   String
  type_name:   String
  year:        i16                      // 当前选中年份
  general:     (i16, i16, i16, i16)    // (start_month, start_day, end_month, end_day)
  existing:    Option<ActualSeasonalPickDto>  // 已有记录（编辑模式）
  on_saved:    Callback<()>             // 保存成功后回调（触发外层 actuals_resource.refetch()）
```

**日期预填充逻辑：**

```
- 无已有记录（新建）时：
    actual_start 默认 = "{year}-{start_month:02}-{start_day:02}"
    actual_end   默认 = 若 end_month < start_month（跨年）→ "{year+1}-{end_month:02}-{end_day:02}"
                        否则 → "{year}-{end_month:02}-{end_day:02}"
- 有已有记录（编辑）时：直接回填 existing.actual_start / actual_end
```

**表单字段：**

```
┌─────────────────────────────────────────────┐
│  Confirm Actual Dates — 2026                │
│  Strawberry Picking · Berry Farm Auckland   │
│                                             │
│  Start Date  [2026-11-08]  (type="date")    │
│  End Date    [2027-02-16]  (type="date")    │
│  Notes       [            ]  (optional)     │
│                                             │
│  [Cancel]                     [Save]        │
└─────────────────────────────────────────────┘
```

使用现有 `<Dialog>` 组件（`components/ui/dialog.rs`）和 `<Input>` 组件。

**Save 动作：**

```rust
let save_action = Action::new(move |_: &()| {
    let pid    = picking_id.clone();
    let y      = year;
    let start  = actual_start.get();
    let end    = actual_end.get();
    let n      = notes.get();
    async move {
        upsert_actual_seasonal_pick(pid, y, start, end, n).await
    }
});

// 监听 action 结果：成功 → open.set(false); on_saved.call(());
```

**已有记录的 Delete 按钮：**

编辑模式下在 Dialog 底部左侧显示 `Delete` 按钮（`ButtonVariant::Destructive`）。
点击后调用 `delete_actual_seasonal_pick(existing.id)`，成功后 `open.set(false); on_saved.call(())`.

---

## 七、文件改动清单

| 类型 | 文件 | 改动内容 |
|------|------|----------|
| Migration | `migrations/<new>_seasonal_picking_actuals.sql` | 新建表 seasonal_picking_actuals |
| Entity | `explonz_shared/src/entity/seasonal_picking_actuals.rs` | 新建（codegen 后手工确认）|
| Entity | `explonz_shared/src/entity/prelude.rs` | 导出 SeasonalPickingActuals |
| Entity | `explonz_bnd/src/entity/seasonal_picking_actuals.rs` | 同步新建 |
| Entity | `explonz_bnd/src/entity/prelude.rs` | 同步导出 |
| DTO | `explonz_shared/src/common/dto.rs` | 新增 ActualSeasonalPickDto |
| Service | `src/service/spots.rs` | 新增 3 个 service fn |
| API DTO | `src/api/spots/dto.rs` | 新增 UpsertActualSeasonalPickRequest |
| Handler | `src/api/spots/handler.rs` | 新增 3 个 handler fn |
| Routes | `src/api/spots/mod.rs` | 注册 3 条新路由 |
| Server fn | `explonz_admin/src/server/spots.rs` | 新增 3 个 server fn |
| UI 页面 | `explonz_admin/src/pages/spots/seasonal_picks_list.rs` | 添加年份选择器、actuals_resource、Actual Dates 列、Confirm/Edit 按钮 |
| UI 组件 | `explonz_admin/src/pages/spots/actual_pick_confirm_dialog.rs` | 新建 Dialog 组件 |
| UI mod | `explonz_admin/src/pages/spots/mod.rs` | `pub mod actual_pick_confirm_dialog;` |

---

## 八、实施顺序

```
Step 1: 开发者手动执行 Migration，生成 Entity（seasonal_picking_actuals）
Step 2: 同步两份 Entity（explonz_shared + explonz_bnd），更新 prelude.rs
Step 3: 添加 ActualSeasonalPickDto（dto.rs）
Step 4: 实现 Service（upsert / get_by_year / delete）
Step 5: 实现 Handler + 注册路由
Step 6: 实现 Admin Server Functions
Step 7: 实现 ActualPickConfirmDialog 组件
Step 8: 改造 SeasonalPicksList（年份选择器 + actuals_resource + 新列 + 按钮联动）
Step 9: cargo check --workspace 验证编译
Step 10: cargo leptos build --package explonz_admin 验证前端构建
```

---

## 九、边界条件与注意事项

1. **跨年日期**：`actual_end` 年份可能比 `actual_start` 晚一年（南半球草莓季跨年）。
   使用 `DATE` 类型天然支持，无需额外标志。预填充逻辑需检测 `end_month < start_month`。

2. **通用季节窗口（general）与实际日期（actual）的关系**：两者独立存储，互不覆盖。
   `seasonal_pickings` 表不修改，`actual_start/end` 可以不在 general 窗口内（如提前或延迟开放）。

3. **Admin UI 数据合并**：`actuals_map` 以 `picking_id` 为 key，每个 picking_id 在同一年只有一条，
   因此 `HashMap<String, ActualSeasonalPickDto>` 已足够。

4. **Leptos Suspense 规则**：两个 Resource 合并在同一 `<Suspense>` view 闭包内读取，
   不得在 `Effect` 中读取任一 Resource，遵循 CLAUDE.md Suspense 传播规则。

5. **乐观并发**：upsert 使用 find+write 两步，极低概率并发冲突时 UNIQUE 约束会返回 DB 错误，
   后端 map 为 `ApiError::BizError("Duplicate confirm for this year")`，前端展示 toast 即可。

6. **年份范围**：前端年份选择器只生成有限选项（当前年 ± 2），不需要任何年份均可查询，
   但后端接口对 year 值无额外限制（由业务判断）。

---

## 十、参考实现代码

### 10.1 DTO（`explonz_shared/src/common/dto.rs`）

在现有 struct 定义区（`SeasonalPickingTypeWithSpotsDto` 之前）插入：

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActualSeasonalPickDto {
    pub id: String,
    pub picking_id: String,
    pub year: i16,
    pub actual_start: String, // "YYYY-MM-DD"
    pub actual_end: String,   // "YYYY-MM-DD"，可跨年
    pub notes: String,
}
```

---

### 10.2 Admin Server Functions（`explonz_admin/src/server/spots.rs`）

import 中新增 `ActualSeasonalPickDto`：

```rust
use explonz_shared::common::dto::{
    ActualSeasonalPickDto, SeasonalPickingTypeDto, SeasonalPickingTypeWithSpotsDto,
    SeasonalPickingsDto, SpotDto,
};
```

追加三个 server fn：

```rust
// 按年份获取所有已确认的实际日期
#[server(GetActualsByYear, "/api")]
pub async fn get_actuals_by_year(
    year: i16,
) -> Result<Vec<ActualSeasonalPickDto>, ServerFnError> {
    let token = crate::server::extract_token().await?;
    let backend_url = crate::server::backend_url();

    let resp = reqwest::Client::new()
        .get(format!(
            "{backend_url}/api/spots/seasonal_pickings/actuals/by_year"
        ))
        .query(&[("year", year.to_string())])
        .bearer_auth(&token)
        .send()
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    if !resp.status().is_success() {
        let msg = resp.text().await.unwrap_or_default();
        return Err(ServerFnError::new(format!("Backend error: {msg}")));
    }

    let parsed: ApiResp<Vec<ActualSeasonalPickDto>> = resp
        .json()
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    parsed
        .data
        .ok_or_else(|| ServerFnError::new("No data returned"))
}

// Upsert 某条 picking 在指定年份的实际开放日期
#[server(UpsertActualSeasonalPick, "/api")]
pub async fn upsert_actual_seasonal_pick(
    picking_id: String,
    year: i16,
    actual_start: String,
    actual_end: String,
    notes: String,
) -> Result<ActualSeasonalPickDto, ServerFnError> {
    let token = crate::server::extract_token().await?;
    let backend_url = crate::server::backend_url();

    let body = serde_json::json!({
        "year": year,
        "actual_start": actual_start,
        "actual_end": actual_end,
        "notes": notes,
    });

    let resp = reqwest::Client::new()
        .post(format!(
            "{backend_url}/api/spots/seasonal_pickings/{picking_id}/actuals/new"
        ))
        .bearer_auth(&token)
        .json(&body)
        .send()
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    if !resp.status().is_success() {
        let msg = resp.text().await.unwrap_or_default();
        return Err(ServerFnError::new(format!("Backend error: {msg}")));
    }

    let parsed: ApiResp<ActualSeasonalPickDto> = resp
        .json()
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    parsed
        .data
        .ok_or_else(|| ServerFnError::new("No data returned"))
}

// 删除一条实际日期记录
#[server(DeleteActualSeasonalPick, "/api")]
pub async fn delete_actual_seasonal_pick(actual_id: String) -> Result<(), ServerFnError> {
    let token = crate::server::extract_token().await?;
    let backend_url = crate::server::backend_url();

    let resp = reqwest::Client::new()
        .delete(format!(
            "{backend_url}/api/spots/seasonal_pickings/actuals/{actual_id}"
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
```

---

### 10.3 ActualPickConfirmDialog（新建 `actual_pick_confirm_dialog.rs`）

```rust
use crate::components::ui::button::{Button, ButtonSize, ButtonVariant};
use crate::components::ui::dialog::{
    Dialog, DialogBody, DialogClose, DialogContent, DialogDescription, DialogFooter, DialogHeader,
    DialogTitle, DialogTrigger,
};
use crate::components::ui::input::{Input, InputType};
use crate::components::ui::label::Label;
use crate::server::spots::{delete_actual_seasonal_pick, upsert_actual_seasonal_pick};
use explonz_shared::common::dto::ActualSeasonalPickDto;
use leptos::html;
use leptos::prelude::*;

#[component]
pub fn ActualPickConfirmDialog(
    picking_id: String,
    spot_name: String,
    type_name: String,
    year: i16,
    general_start_month: i16,
    general_start_day: i16,
    general_end_month: i16,
    general_end_day: i16,
    // 注意：不加 #[prop(optional)]，直接传 Option 值
    existing: Option<ActualSeasonalPickDto>,
    on_saved: Callback<()>,
) -> impl IntoView {
    let is_edit = existing.is_some();

    // 默认日期：跨年时 end_year = year + 1
    let default_start = format!("{year}-{general_start_month:02}-{general_start_day:02}");
    let end_year = if general_end_month < general_start_month {
        year + 1
    } else {
        year
    };
    let default_end = format!("{end_year}-{general_end_month:02}-{general_end_day:02}");

    let actual_start = RwSignal::new(
        existing
            .as_ref()
            .map(|e| e.actual_start.clone())
            .unwrap_or(default_start),
    );
    let actual_end = RwSignal::new(
        existing
            .as_ref()
            .map(|e| e.actual_end.clone())
            .unwrap_or(default_end),
    );
    let notes = RwSignal::new(
        existing
            .as_ref()
            .map(|e| e.notes.clone())
            .unwrap_or_default(),
    );
    let error_msg = RwSignal::new(Option::<String>::None);

    let existing_id = existing.map(|e| e.id);

    // 用于编程式关闭 dialog 的隐藏按钮引用
    let close_ref: NodeRef<html::Button> = NodeRef::new();

    let picking_id_for_save = picking_id.clone();
    let save_action = Action::new(move |_: &()| {
        let pid = picking_id_for_save.clone();
        let y = year;
        let start = actual_start.get_untracked();
        let end = actual_end.get_untracked();
        let n = notes.get_untracked();
        async move { upsert_actual_seasonal_pick(pid, y, start, end, n).await }
    });

    let delete_action = Action::new(move |_: &()| {
        let aid = existing_id.clone().unwrap_or_default();
        async move { delete_actual_seasonal_pick(aid).await }
    });

    // 监听 save 结果
    Effect::new(move |_| match save_action.value().get() {
        Some(Ok(_)) => {
            error_msg.set(None);
            on_saved.run(());
            // 编程式关闭 dialog（仅 WASM）
            #[cfg(target_arch = "wasm32")]
            if let Some(btn) = close_ref.get() {
                let _ = btn.click();
            }
        }
        Some(Err(e)) => {
            error_msg.set(Some(e.to_string()));
        }
        None => {}
    });

    // 监听 delete 结果
    Effect::new(move |_| match delete_action.value().get() {
        Some(Ok(_)) => {
            error_msg.set(None);
            on_saved.run(());
            #[cfg(target_arch = "wasm32")]
            if let Some(btn) = close_ref.get() {
                let _ = btn.click();
            }
        }
        Some(Err(e)) => {
            error_msg.set(Some(e.to_string()));
        }
        None => {}
    });

    let is_pending = move || save_action.pending().get() || delete_action.pending().get();

    let trigger_variant = if is_edit {
        ButtonVariant::Ghost
    } else {
        ButtonVariant::Outline
    };
    let trigger_label = if is_edit { "Edit" } else { "Confirm" };

    view! {
        <Dialog>
            <DialogTrigger
                variant=trigger_variant
                size=ButtonSize::Sm
                attr:class="text-xs h-7"
            >
                {trigger_label}
            </DialogTrigger>
            <DialogContent class="sm:max-w-md">
                // 隐藏按钮：供 Effect 编程式触发 dialog 关闭
                // Dialog JS 会将 [data-dialog-close] 元素注册为关闭触发器
                <button
                    type="button"
                    class="hidden"
                    attr:data-dialog-close="close"
                    node_ref=close_ref
                />
                <DialogHeader>
                    <DialogTitle>{format!("Actual Dates — {year}")}</DialogTitle>
                    <DialogDescription>
                        {format!("{type_name} · {spot_name}")}
                    </DialogDescription>
                </DialogHeader>
                <DialogBody>
                    <div class="grid gap-4">
                        <div class="grid gap-1.5">
                            <Label html_for="actual_start_input">"Start Date"</Label>
                            <Input
                                r#type=InputType::Date
                                id="actual_start_input"
                                name="actual_start"
                                bind_value=actual_start
                            />
                        </div>
                        <div class="grid gap-1.5">
                            <Label html_for="actual_end_input">"End Date"</Label>
                            <Input
                                r#type=InputType::Date
                                id="actual_end_input"
                                name="actual_end"
                                bind_value=actual_end
                            />
                        </div>
                        <div class="grid gap-1.5">
                            <Label html_for="notes_input">
                                "Notes "
                                <span class="font-normal text-muted-foreground">"(optional)"</span>
                            </Label>
                            <Input
                                id="notes_input"
                                name="notes"
                                placeholder="e.g. Confirmed by farm owner"
                                bind_value=notes
                            />
                        </div>
                        {move || {
                            error_msg
                                .get()
                                .map(|e| view! { <p class="text-sm text-destructive">{e}</p> })
                        }}
                    </div>
                </DialogBody>
                <DialogFooter>
                    {is_edit.then(|| {
                        view! {
                            <Button
                                variant=ButtonVariant::Destructive
                                size=ButtonSize::Sm
                                attr:class="mr-auto"
                                attr:disabled=move || is_pending()
                                on:click=move |_| {
                                    delete_action.dispatch(());
                                }
                            >
                                "Delete"
                            </Button>
                        }
                    })}
                    <DialogClose variant=ButtonVariant::Ghost>"Cancel"</DialogClose>
                    <Button
                        attr:disabled=move || is_pending()
                        on:click=move |_| {
                            save_action.dispatch(());
                        }
                    >
                        {move || if save_action.pending().get() { "Saving..." } else { "Save" }}
                    </Button>
                </DialogFooter>
            </DialogContent>
        </Dialog>
    }
}
```

---

### 10.4 SeasonalPicksList 完整改造（`seasonal_picks_list.rs`）

**编译注意事项（已验证）：**
- `actuals_map` 必须用 `Arc::new(...)` 包装，在 `|pt|` 和 `|spot|` 闭包中用 `Arc::clone` 共享
  （原因：Leptos SSR 要求闭包 `Send`，`Rc` 不满足；迭代闭包为 `FnMut`，不能 move HashMap）
- `existing: Option<ActualSeasonalPickDto>` 传给组件时**不加** `#[prop(optional)]`，
  否则 Leptos 会把传入值再包一层 `Some`，导致类型不匹配

```rust
use std::collections::HashMap;
use std::sync::Arc;

use crate::components::ui::accordion::{
    Accordion, AccordionContent, AccordionItem, AccordionTrigger,
};
use crate::pages::spots::actual_pick_confirm_dialog::ActualPickConfirmDialog;
use crate::server::spots::{get_actuals_by_year, get_all_seasonal_pickings_with_spots};
use explonz_shared::common::dto::ActualSeasonalPickDto;
use leptos::prelude::*;

use super::month_short;

/// "YYYY-MM-DD" → "Nov 8"
fn format_date_short(date_str: &str) -> String {
    let parts: Vec<&str> = date_str.split('-').collect();
    if parts.len() != 3 {
        return date_str.to_string();
    }
    let month = parts[1]
        .parse::<i16>()
        .ok()
        .map(month_short)
        .unwrap_or("?");
    let day = parts[2].trim_start_matches('0');
    format!("{} {}", month, day)
}

/// 获取当前年份（SSR 与 WASM 均可用，基于 SystemTime 近似计算）
fn current_year() -> i16 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    // 365.2425 天/年 × 86400 秒/天 ≈ 31_556_952
    (1970 + secs / 31_556_952) as i16
}

#[component]
pub fn SeasonalPicksList() -> impl IntoView {
    let default_year = current_year();
    let year_range: Vec<i16> = (default_year - 2..=default_year + 2).collect();
    let selected_year = RwSignal::new(default_year);

    let picks_resource = Resource::new(|| (), |_| get_all_seasonal_pickings_with_spots());
    let actuals_resource = Resource::new(
        move || selected_year.get(),
        |year| get_actuals_by_year(year),
    );

    view! {
        <div class="flex flex-col gap-3">
            // 顶部 Header：标题 + 年份选择器（Suspense 外，始终可见）
            <div class="flex items-center justify-between">
                <h2 class="text-base font-semibold">"Seasonal Picks"</h2>
                <select
                    class="text-xs border rounded px-2 py-1 bg-background text-foreground"
                    on:change=move |e| {
                        let val = event_target_value(&e)
                            .parse::<i16>()
                            .unwrap_or(default_year);
                        selected_year.set(val);
                    }
                >
                    {year_range
                        .iter()
                        .map(|&y| {
                            view! {
                                <option
                                    value=y.to_string()
                                    selected=move || selected_year.get() == y
                                >
                                    {y.to_string()}
                                </option>
                            }
                        })
                        .collect_view()}
                </select>
            </div>

            // picks_resource 与 actuals_resource 在同一 Suspense 内读取（防止 pending 冒泡到 AuthGuard）
            <Suspense fallback=|| {
                view! { <div class="text-sm text-muted-foreground p-4">"Loading..."</div> }
            }>
                {move || {
                    let picks = picks_resource.get();
                    let actuals = actuals_resource.get();

                    let Some(Ok(picks)) = picks else {
                        return view! { <div /> }.into_any();
                    };

                    // 构建 picking_id → ActualSeasonalPickDto 映射
                    // 用 Arc 包装以便在多个迭代闭包（FnMut）中共享，同时满足 SSR 的 Send 要求
                    let actuals_map: Arc<HashMap<String, ActualSeasonalPickDto>> =
                        Arc::new(
                            actuals
                                .and_then(|r| r.ok())
                                .unwrap_or_default()
                                .into_iter()
                                .map(|a| (a.picking_id.clone(), a))
                                .collect(),
                        );

                    let type_count = picks.len();
                    let year = selected_year.get();

                    view! {
                        <div class="flex flex-col gap-3">
                            <div class="flex items-center justify-end">
                                <span class="text-xs text-muted-foreground">
                                    {format!("{} types", type_count)}
                                </span>
                            </div>

                            <div class="grid grid-cols-[1fr_auto_auto] gap-4 px-3 py-1.5 text-xs font-medium text-muted-foreground border-b">
                                <span>"Type"</span>
                                <span class="w-40 text-center">"Season"</span>
                                <span class="w-16 text-center">"Spots"</span>
                            </div>

                            <Accordion>
                                {picks
                                    .into_iter()
                                    .map(move |pt| {
                                        // 每次迭代 clone Arc（廉价引用计数）
                                        let actuals_map = Arc::clone(&actuals_map);
                                        let season = format!(
                                            "{} {} – {} {}",
                                            month_short(pt.start_month),
                                            pt.start_day,
                                            month_short(pt.end_month),
                                            pt.end_day,
                                        );
                                        let spot_count = pt.spots.len();
                                        let type_name = pt.type_name.clone();
                                        let start_month = pt.start_month;
                                        let start_day = pt.start_day;
                                        let end_month = pt.end_month;
                                        let end_day = pt.end_day;

                                        view! {
                                            <AccordionItem>
                                                <AccordionTrigger class="py-1">
                                                    <div class="grid grid-cols-[1fr_auto_auto] gap-4 w-full items-center">
                                                        <span class="text-sm font-medium">
                                                            {pt.type_name}
                                                        </span>
                                                        <span class="w-40 text-center text-sm text-muted-foreground">
                                                            {season}
                                                        </span>
                                                        <span class="w-16 text-center">
                                                            <span class="inline-flex items-center justify-center rounded-full bg-muted px-2 text-xs font-medium">
                                                                {spot_count}
                                                            </span>
                                                        </span>
                                                    </div>
                                                </AccordionTrigger>

                                                <AccordionContent>
                                                    <div class="rounded-md border overflow-hidden">
                                                        <table class="w-full text-sm">
                                                            <thead>
                                                                <tr class="border-b bg-muted/40">
                                                                    <th class="px-3 py-2 text-left text-xs font-medium text-muted-foreground">
                                                                        "Spot"
                                                                    </th>
                                                                    <th class="px-3 py-2 text-left text-xs font-medium text-muted-foreground">
                                                                        "Location"
                                                                    </th>
                                                                    <th class="px-3 py-2 text-left text-xs font-medium text-muted-foreground">
                                                                        "Season Picks"
                                                                    </th>
                                                                    <th class="px-3 py-2 text-left text-xs font-medium text-muted-foreground">
                                                                        "Actual Dates"
                                                                    </th>
                                                                    <th class="px-3 py-2 text-right text-xs font-medium text-muted-foreground">
                                                                        "Rating"
                                                                    </th>
                                                                    <th class="px-3 py-2 text-right text-xs font-medium text-muted-foreground">
                                                                        ""
                                                                    </th>
                                                                </tr>
                                                            </thead>
                                                            <tbody>
                                                                {pt
                                                                    .spots
                                                                    .into_iter()
                                                                    .map(move |spot| {
                                                                        let existing = actuals_map
                                                                            .get(&spot.picking_id)
                                                                            .cloned();
                                                                        let picking_id =
                                                                            spot.picking_id.clone();
                                                                        let spot_name_str =
                                                                            spot.spot_name.clone();
                                                                        let type_name_str =
                                                                            type_name.clone();

                                                                        let actual_display =
                                                                            match &existing {
                                                                                Some(a) => format!(
                                                                                    "{} – {}",
                                                                                    format_date_short(
                                                                                        &a.actual_start,
                                                                                    ),
                                                                                    format_date_short(
                                                                                        &a.actual_end,
                                                                                    ),
                                                                                ),
                                                                                None => {
                                                                                    "—".to_string()
                                                                                }
                                                                            };
                                                                        let has_actual =
                                                                            existing.is_some();

                                                                        view! {
                                                                            <tr class="border-b last:border-0 hover:bg-muted/30 transition-colors">
                                                                                <td class="px-3 py-2 font-medium">
                                                                                    {spot.spot_name}
                                                                                </td>
                                                                                <td class="px-3 py-2 text-muted-foreground">
                                                                                    {spot.location}
                                                                                </td>
                                                                                <td class="px-3 py-2 text-muted-foreground">
                                                                                    {spot.created_at}
                                                                                </td>
                                                                                <td class="px-3 py-2">
                                                                                    <span class=if has_actual {
                                                                                        "text-xs text-green-600 font-medium"
                                                                                    } else {
                                                                                        "text-xs text-muted-foreground"
                                                                                    }>
                                                                                        {actual_display}
                                                                                    </span>
                                                                                </td>
                                                                                <td class="px-3 py-2 text-right">
                                                                                    <span class="inline-flex items-center gap-1 text-amber-500 font-medium">
                                                                                        "★ "
                                                                                        {spot.rating}
                                                                                    </span>
                                                                                </td>
                                                                                <td class="px-3 py-2">
                                                                                    <div class="flex justify-end">
                                                                                        <ActualPickConfirmDialog
                                                                                            picking_id=picking_id
                                                                                            spot_name=spot_name_str
                                                                                            type_name=type_name_str
                                                                                            year=year
                                                                                            general_start_month=start_month
                                                                                            general_start_day=start_day
                                                                                            general_end_month=end_month
                                                                                            general_end_day=end_day
                                                                                            existing=existing
                                                                                            on_saved=Callback::new(move |_: ()| {
                                                                                                actuals_resource.refetch();
                                                                                            })
                                                                                        />
                                                                                    </div>
                                                                                </td>
                                                                            </tr>
                                                                        }
                                                                    })
                                                                    .collect_view()}
                                                            </tbody>
                                                        </table>
                                                    </div>
                                                </AccordionContent>
                                            </AccordionItem>
                                        }
                                    })
                                    .collect_view()}
                            </Accordion>
                        </div>
                    }
                        .into_any()
                }}
            </Suspense>
        </div>
    }
}
```

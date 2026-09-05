# Spot 标签（Labels）技术方案

> 版本：v2.1
> 更新说明：引入独立 `spot_labels` 表及 `spot_label_assignments` 关联表，通过独立 CRUD 接口维护标签库；Admin 新增"Labels"菜单；Addition/Edit 页面改为从标签库选标签。`spot_labels` 字段名调整：`type` → `name`，`label` → `description`。

---

## 1. 背景与目标

**当前问题：** Attributes 区域为原始 JSON textarea，用户需手动输入 JSON，体验差，标签无法复用或统一管理。

**目标：**
1. 新增独立 `spot_labels` 标签库表，通过 CRUD 接口管理
2. Admin 侧边栏增加"Labels"菜单入口，供管理员维护标签库
3. 创建/编辑 Spot 时，从标签库选取标签，写入 `spot_label_assignments` 关联表
4. 后端 API 通过 JOIN 返回 Spot 的完整标签信息，移动端无感知

---

## 2. 数据库 Schema 变更

### 2.1 新增表：`spot_labels`（标签库主表）

```sql
-- ---------------------------------------------------------------------------
-- 表：spot_labels（标签库主表）
--
-- 由管理员通过后台 CRUD 接口维护。
-- name        — 英文唯一标识符，如 family_friendly（供移动端逻辑判断使用）
-- description — 用户可读说明，如 "Family Friendly"（展示用）
-- icon        — 图标名称字符串，对应前端 icons crate 中的组件名，如 "Users"
-- ---------------------------------------------------------------------------

CREATE TABLE spot_labels (
    id          UUID        PRIMARY KEY DEFAULT uuidv7(),
    name        TEXT        NOT NULL,
    description TEXT        NOT NULL,
    icon        TEXT        NOT NULL DEFAULT 'Tag',
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    -- name 全局唯一，防止同义重复标签
    CONSTRAINT uq_spot_labels_name UNIQUE (name)
);

CREATE TRIGGER trg_spot_labels_updated_at
    BEFORE UPDATE ON spot_labels
    FOR EACH ROW EXECUTE FUNCTION fn_set_updated_at();
```

### 2.2 新增表：`spot_label_assignments`（Spot 与标签多对多关联）

```sql
-- ---------------------------------------------------------------------------
-- 表：spot_label_assignments（Spot 与标签的关联表）
--
-- 一个 Spot 可关联多个标签；一个标签可被多个 Spot 使用。
-- 删除 Spot 时级联删除关联；删除标签时级联删除关联（谨慎操作）。
-- ---------------------------------------------------------------------------

CREATE TABLE spot_label_assignments (
    spot_id  UUID NOT NULL REFERENCES spots(id)        ON DELETE CASCADE,
    label_id UUID NOT NULL REFERENCES spot_labels(id)  ON DELETE CASCADE,

    PRIMARY KEY (spot_id, label_id)
);

CREATE INDEX idx_spot_label_assignments_spot_id  ON spot_label_assignments (spot_id);
CREATE INDEX idx_spot_label_assignments_label_id ON spot_label_assignments (label_id);
```

### 2.3 `spots.attributes` 字段处理策略

现有 `spots.attributes JSONB` 字段暂时保留，**不立即删除**，以保证迁移期间的兼容性。

- **新写入路径**：创建/更新 Spot 时，标签写入 `spot_label_assignments`；`spots.attributes` 置为 `'[]'` 或不再维护。
- **读取路径**：后端 `GET /spots/:id` 及 `GET /spots` 通过 JOIN `spot_label_assignments` + `spot_labels` 组装标签数组，以 `attributes` 字段返回给客户端（格式不变），移动端无需修改。
- **未来版本**：待全量数据迁移完成后，可通过 migration 删除 `spots.attributes` 列。

### 2.4 历史数据迁移 SQL（可选，手动执行）

若需将已有 JSONB 标签数据迁移进新表，执行以下脚本：

```sql
-- Step 1：从现有 spots.attributes 中提取去重标签写入 spot_labels
-- 注：JSON 中的旧字段名 "type"/"label" 映射到新列名 name/description
INSERT INTO spot_labels (name, description, icon)
SELECT DISTINCT
    elem->>'type'                        AS name,
    elem->>'label'                       AS description,
    COALESCE(elem->>'icon', 'Tag')       AS icon
FROM spots,
     jsonb_array_elements(attributes) AS elem
WHERE elem->>'type' IS NOT NULL
  AND elem->>'label' IS NOT NULL
ON CONFLICT (name) DO NOTHING;

-- Step 2：写入关联表
INSERT INTO spot_label_assignments (spot_id, label_id)
SELECT
    s.id                                 AS spot_id,
    sl.id                                AS label_id
FROM spots s,
     jsonb_array_elements(s.attributes) AS elem
JOIN spot_labels sl ON sl.name = elem->>'type'
ON CONFLICT DO NOTHING;
```

---

## 3. 后端 API 变更（Backend API Server）

### 3.1 新增标签库 CRUD 接口

| 方法   | 路径                  | 描述             |
|--------|-----------------------|------------------|
| GET    | `/api/labels`         | 获取标签列表     |
| POST   | `/api/labels`         | 创建新标签       |
| PUT    | `/api/labels/:id`     | 更新标签         |
| DELETE | `/api/labels/:id`     | 删除标签         |

Request Body（POST/PUT）：
```json
{
  "name":        "family_friendly",
  "description": "Family Friendly",
  "icon":        "Users"
}
```

### 3.2 修改 Spot 创建/更新接口

**`POST /api/spots/new`** 请求体变更：

移除 `attributes` JSONB 字段，新增 `label_ids` 数组：

```json
{
  "name":             "Spot Name",
  "label_ids":        ["uuid-1", "uuid-2"],
  ...
}
```

后端处理：
1. 创建 Spot 记录（`spots.attributes` 写入 `'[]'`）
2. 批量插入 `spot_label_assignments`

**`GET /api/spots` / `GET /api/spots/:id`** 响应变更：

后端通过 JOIN 组装标签，`attributes` 字段格式与现有兼容：
```json
{
  "attributes": [
    { "id": "uuid-1", "name": "family_friendly", "description": "Family Friendly", "icon": "Users" }
  ]
}
```

> 新增 `id` 字段（`spot_labels.id`），供前端回填已选标签状态使用。

### 3.3 `SpotLabelDto`（explonz_shared）

```rust
// explonz_shared/src/common/dto.rs 新增
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SpotLabelDto {
    pub id:          Uuid,
    pub name:        String,
    pub description: String,
    pub icon:        String,
}

// SpotDto.attributes 类型更新
pub struct SpotDto {
    // ... 其他字段不变 ...
    pub attributes: Vec<SpotLabelDto>,   // 原来是 serde_json::Value
}
```

---

## 4. Admin Server Functions（explonz_admin/src/server/）

### 4.1 新增 `server/labels.rs`

```rust
// 获取所有标签
#[server(GetLabels, "/api")]
pub async fn get_labels() -> Result<Vec<SpotLabelDto>, ServerFnError>

// 创建标签
#[server(CreateLabel, "/api")]
pub async fn create_label(
    name:        String,
    description: String,
    icon:        String,
) -> Result<SpotLabelDto, ServerFnError>

// 更新标签
#[server(UpdateLabel, "/api")]
pub async fn update_label(
    id:          String,
    name:        String,
    description: String,
    icon:        String,
) -> Result<SpotLabelDto, ServerFnError>

// 删除标签
#[server(DeleteLabel, "/api")]
pub async fn delete_label(id: String) -> Result<(), ServerFnError>
```

### 4.2 修改 `server/spots.rs`

`create_spot` 签名变更：
- 移除：`attributes_json: String`
- 新增：`label_ids: Vec<String>`（多个同名 hidden input 自动反序列化）

```rust
#[server(CreateSpot, "/api")]
pub async fn create_spot(
    name:              String,
    location:          String,
    latitude:          f64,
    longitude:         f64,
    description:       String,
    photo_urls:        Vec<String>,
    label_ids:         Vec<String>,   // [CHANGED] 替代 attributes_json
    phone:             Option<String>,
    website:           Option<String>,
    opening_hours_json: String,
) -> Result<SpotDto, ServerFnError>
```

---

## 5. 图标方案设计

### 5.1 存储与渲染原理

`icons` crate 中图标为 Leptos 组件（如 `icons::Users`），无法在运行时通过字符串动态构造。

解决方案：维护静态对照表，通过 `match` 分发渲染。

### 5.2 预设图标列表（`spot_label.rs`）

```rust
pub const AVAILABLE_ICONS: &[(&str, &str)] = &[
    ("Tag",          "标签"),
    ("Users",        "人群"),
    ("Star",         "星级"),
    ("MapPin",       "地点"),
    ("TreePine",     "自然"),
    ("Flame",        "热门"),
    ("Waves",        "水域"),
    ("PawPrint",     "宠物"),
    ("Baby",         "亲子"),
    ("Accessibility","无障碍"),
    ("Parking",      "停车"),
    ("Coffee",       "餐饮"),
    ("Camera",       "摄影"),
    ("Sunset",       "景观"),
    ("Tent",         "露营"),
    ("Mountain",     "山地"),
    ("Bike",         "骑行"),
    ("FootPrints",   "步行"),
    ("Clock",        "时间"),
    ("Wifi",         "网络"),
];

pub fn render_icon(name: &str) -> AnyView {
    match name {
        "Tag"          => view! { <icons::Tag /> }.into_any(),
        "Users"        => view! { <icons::Users /> }.into_any(),
        "Star"         => view! { <icons::Star /> }.into_any(),
        "MapPin"       => view! { <icons::MapPin /> }.into_any(),
        "TreePine"     => view! { <icons::TreePine /> }.into_any(),
        "Flame"        => view! { <icons::Flame /> }.into_any(),
        "Waves"        => view! { <icons::Waves /> }.into_any(),
        "PawPrint"     => view! { <icons::PawPrint /> }.into_any(),
        "Baby"         => view! { <icons::Baby /> }.into_any(),
        "Accessibility"=> view! { <icons::Accessibility /> }.into_any(),
        "Parking"      => view! { <icons::Parking /> }.into_any(),
        "Coffee"       => view! { <icons::Coffee /> }.into_any(),
        "Camera"       => view! { <icons::Camera /> }.into_any(),
        "Sunset"       => view! { <icons::Sunset /> }.into_any(),
        "Tent"         => view! { <icons::Tent /> }.into_any(),
        "Mountain"     => view! { <icons::Mountain /> }.into_any(),
        "Bike"         => view! { <icons::Bike /> }.into_any(),
        "FootPrints"   => view! { <icons::FootPrints /> }.into_any(),
        "Clock"        => view! { <icons::Clock /> }.into_any(),
        "Wifi"         => view! { <icons::Wifi /> }.into_any(),
        _              => view! { <icons::Tag /> }.into_any(),
    }
}
```

> 使用前需确认 `icons` crate 中包含上述图标，不存在的替换为近似图标。

---

## 6. UI 交互设计

### 6.1 Labels 管理页面（新增）

路由：`/labels`（侧边栏"Labels"菜单）

```
┌──────────────────────────────────────────────────────────┐
│ Labels                                    [+ 新建标签]   │
├──────────────────────────────────────────────────────────┤
│  图标    简称                说明              操作      │
│  [👥]   family_friendly    Family Friendly   [编辑][删] │
│  [⭐]   hot_spot           Hot Spot          [编辑][删] │
│  [🐾]   pet_friendly       Pet Friendly      [编辑][删] │
│  ...                                                      │
└──────────────────────────────────────────────────────────┘
```

点击"新建标签"打开 Sheet 侧边栏或 inline 表单：
```
┌──────────────────────────────────┐
│ 新建标签                          │
│  标签简称:  [family_friendly___] │  ← 英文，仅字母/数字/下划线
│  标签说明:  [Family Friendly___] │
│  标签图标:  [👥 Users ▼ 选择]   │  ← 点击展开图标选择器
│                                  │
│                [取消]  [创建标签] │
└──────────────────────────────────┘
```

图标选择器（inline grid，点击展开）：
```
┌──────────────────────────────────┐
│  [👥] [⭐] [📍] [🌿] [🔥] [🌊] │
│  [🐾] [👶] [♿] [🅿️] [☕] [📷] │
│  [🌅] [⛺] [⛰] [🚲] [👣] [🕐] │
│  [📶] [🏷]                       │
└──────────────────────────────────┘
```

### 6.2 Spot 创建页 Attributes 区域（改造 addition.rs）

```
┌─────────────────────────────────────────────────────┐
│ 标签                                                 │
│                                                      │
│  [👥 Family Friendly ×]  [⭐ Hot Spot ×]            │
│                                                      │
│  [+ 从标签库添加]                                    │
└─────────────────────────────────────────────────────┘
```

点击"从标签库添加"后，在区域下方展开标签库面板：
```
┌─────────────────────────────────────────────────────┐
│ 选择标签（点击添加，再次点击取消）                    │
│                                                      │
│  [👥 Family Friendly]  [⭐ Hot Spot]  [🐾 Pet ...] │
│  [👶 Kid Friendly]     [♿ Accessible] ...           │
│                                                      │
│                                          [完成]      │
└─────────────────────────────────────────────────────┘
```

- 标签库数据来自 `get_labels()` Resource（页面加载时请求）
- 已选中的标签高亮显示（如 `bg-primary` 样式）
- 点击标签 toggle 选中/取消
- 点击"完成"关闭面板

---

## 7. 前端状态管理

### 7.1 `addition.rs` 中的标签状态

```rust
// 标签库（从服务端加载）
let all_labels = Resource::new(|| (), |_| get_labels());

// 当前 Spot 已选标签的 ID 集合
let selected_label_ids: RwSignal<Vec<String>> = RwSignal::new(vec![]);

// 是否展示标签选择面板
let show_label_panel: RwSignal<bool> = RwSignal::new(false);
```

### 7.2 表单提交（隐藏字段）

与 `photo_urls` 机制一致，每个选中的 label_id 生成一个同名 hidden input：

```rust
{move || selected_label_ids.get().into_iter().map(|id| view! {
    <input type="hidden" name="label_ids" value=id />
}).collect_view()}
```

服务端 `create_spot` 的 `label_ids: Vec<String>` 自动反序列化多个同名字段。

### 7.3 验证规则

- 标签为可选项，可以不选任何标签
- 同一标签不允许重复选中（由 toggle 逻辑保证）
- 标签库创建时：`name` 字段仅允许小写字母/数字/下划线，`description` 非空，`icon` 来自 `AVAILABLE_ICONS`

---

## 8. 文件组织

```
explonz_admin/src/
├── pages/
│   ├── mod.rs                       # [CHANGED] 注册 labels 模块
│   ├── spots/
│   │   ├── addition.rs              # [CHANGED] 标签区域改造
│   │   ├── edit.rs                  # [CHANGED] 标签区域改造（回填已选）
│   │   ├── spot_label.rs            # [NEW] SpotLabelDto、render_icon、AVAILABLE_ICONS
│   │   └── spot_label_tsd.md
│   └── labels/                      # [NEW] Labels 管理页面
│       ├── mod.rs
│       └── list.rs                  # Labels 列表 + 新建/编辑/删除
├── server/
│   ├── mod.rs                       # [CHANGED] 注册 labels 模块
│   ├── spots.rs                     # [CHANGED] create_spot 签名
│   └── labels.rs                    # [NEW] get_labels / create_label / update_label / delete_label
└── components/
    └── blocks/
        └── sidenav_routes.rs        # [CHANGED] 新增 Labels 菜单项
```

### `pages/labels/list.rs` 关键组件

```rust
#[component]
pub fn LabelList() -> impl IntoView {
    let labels   = Resource::new(|| (), |_| get_labels());
    let show_form = RwSignal::new(false);   // 控制新建/编辑面板
    let edit_target: RwSignal<Option<SpotLabelDto>> = RwSignal::new(None);
    // ...
}
```

### `sidenav_routes.rs` 新增入口

```rust
// Labels 菜单项（位于 Spots 之后）
SidenavItem { label: "Labels", href: "/labels", icon: Tag }
```

---

## 9. Migration 文件

新建 migration 文件（遵循现有命名约定）：

```
migrations/20260831XXXXXX_spot_labels.sql
```

内容包含 §2.1 和 §2.2 的建表 SQL。历史数据迁移脚本（§2.4）作为独立脚本手动执行，不纳入自动 migration，以便人工审核。

---

## 10. 实现顺序

1. **数据库**：执行 §2.1、§2.2 建表 SQL（新建 migration 文件）
2. **后端 API**：实现 `/api/labels` CRUD 接口；修改 `/api/spots/new` 接受 `label_ids`
3. **`explonz_shared`**：新增 `SpotLabelDto`；更新 `SpotDto.attributes` 类型
4. **`server/labels.rs`**：实现 4 个 server functions
5. **`server/spots.rs`**：更新 `create_spot` 签名
6. **`spot_label.rs`**：`render_icon` + `AVAILABLE_ICONS`
7. **`pages/labels/list.rs`**：Labels 管理页面（列表 + 新建/编辑/删除）
8. **`sidenav_routes.rs`**：添加 Labels 菜单入口
9. **`addition.rs`**：替换 textarea，引入标签选择器
10. **`edit.rs`**：回填已有标签（将 `SpotDto.attributes` 中的 `id` 列表写入 `selected_label_ids`）
11. **数据迁移**：手动执行 §2.4 脚本，迁移历史 JSONB 数据

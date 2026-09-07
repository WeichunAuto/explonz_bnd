// [CHANGED] was: use icons::Trash2;
// now: 新增 CloudUpload 图标用于 Dropzone
use icons::{CloudUpload, Trash2};

use leptos::prelude::*;
use leptos_router::hooks::use_navigate;
use leptos_router::NavigateOptions;
use serde::Serialize;

use crate::components::ui::button::{Button, ButtonVariant};
use crate::components::ui::card::{Card, CardContent, CardHeader, CardTitle};
use crate::components::ui::input::{Input, InputType};
use crate::components::ui::label::Label;
use crate::server::labels::get_labels;
use crate::server::spots::{geocode_location, CreateSpot};
use explonz_shared::icons::LabelIcon;

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
    Done {
        img_id: String,
        url: String,
    },
    Failed(String),
}

// ── [NEW] WASM-only 上传辅助函数 ───────────────────────────────
/// 将单个文件通过 upload_photo server fn 上传，异步更新 status signal
/// [CHANGED] was: 直接用 gloo_net::http::Request::post("/admin/api/photos")
/// now: 调用 upload_photo server fn（服务端再转发给后端 API）
#[cfg(target_arch = "wasm32")]
fn upload_file(file: web_sys::File, status: RwSignal<PhotoStatus>) {
    use crate::server::spots::upload_photo;
    use server_fn::codec::MultipartData;

    leptos::task::spawn_local(async move {
        // 构造 FormData，server fn MultipartFormData 编码会将其序列化后发送
        let form_data = web_sys::FormData::new().unwrap();
        let _ = form_data.append_with_blob("file", &file);

        match upload_photo(MultipartData::from(form_data)).await {
            Ok(resp) => status.set(PhotoStatus::Done {
                img_id: resp.id,
                url: resp.url,
            }),
            Err(e) => status.set(PhotoStatus::Failed(format!("{e}"))),
        }
    });
}

// ── [NEW] WASM-only 批量处理 FileList ──────────────────────────
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
            photos.update(|v| {
                v.push(PhotoItem {
                    id: local_id,
                    status,
                })
            });
            // [CHANGED] was: upload_file(file, status, photos, local_id)
            // now: photos 参数移除（upload_file 只更新 status，删除由外部 photos signal 处理）
            upload_file(file, status);
        }
    }
}

// ───────────────────────────────────────────────────────────────
// label 图标静态分发（与 labels/list.rs 保持一致）
fn render_icon(name: LabelIcon) -> AnyView {
    match name {
        LabelIcon::Tag => view! { <icons::Tag /> }.into_any(),
        LabelIcon::Users => view! { <icons::Users /> }.into_any(),
        LabelIcon::Star => view! { <icons::Star /> }.into_any(),
        LabelIcon::MapPin => view! { <icons::MapPin /> }.into_any(),
        LabelIcon::Flame => view! { <icons::Flame /> }.into_any(),
        LabelIcon::Coffee => view! { <icons::Coffee /> }.into_any(),
        LabelIcon::Camera => view! { <icons::Camera /> }.into_any(),
        LabelIcon::Wifi => view! { <icons::Wifi /> }.into_any(),
        LabelIcon::Clock => view! { <icons::Clock /> }.into_any(),
        LabelIcon::Mountain => view! { <icons::Mountain /> }.into_any(),
        LabelIcon::TreePine => view! { <icons::TreePine /> }.into_any(),
        LabelIcon::Waves => view! { <icons::Waves /> }.into_any(),
        LabelIcon::Baby => view! { <icons::Baby /> }.into_any(),
        LabelIcon::PawPrint => view! { <icons::PawPrint /> }.into_any(),
        LabelIcon::Bike => view! { <icons::Bike /> }.into_any(),
        LabelIcon::Tent => view! { <icons::Tent /> }.into_any(),
        LabelIcon::Sunset => view! { <icons::Sunset /> }.into_any(),
        LabelIcon::Accessibility => view! { <icons::Accessibility /> }.into_any(),
    }
}

// ───────────────────────────────────────────────────────────────

#[component]
pub fn SpotAddition() -> impl IntoView {
    let create_action = ServerAction::<CreateSpot>::new();
    let navigate = use_navigate();
    let navigate_cancel = navigate.clone();

    // ── Labels ────────────────────────────────────────────────────
    let label_fetch_trigger = RwSignal::new(false);
    Effect::new(move |_| {
        label_fetch_trigger.set(true);
    });
    let labels = Resource::new(
        move || label_fetch_trigger.get(),
        move |ready| async move {
            if !ready {
                return Ok(vec![]);
            }
            get_labels().await
        },
    );
    let selected_label_ids: RwSignal<Vec<String>> = RwSignal::new(vec![]);
    let show_dropdown = RwSignal::new(false);
    let search_query: RwSignal<String> = RwSignal::new(String::new());

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
        ("Sunday", DayStatus::Closed),
        ("Monday", DayStatus::Open),
        ("Tuesday", DayStatus::Open),
        ("Wednesday", DayStatus::Open),
        ("Thursday", DayStatus::Open),
        ("Friday", DayStatus::Open),
        ("Saturday", DayStatus::Closed),
    ];
    let schedules: Vec<(&'static str, DaySchedule)> = days
        .into_iter()
        .map(|(name, default_status)| {
            (
                name,
                DaySchedule {
                    status: RwSignal::new(default_status),
                    open_time: RwSignal::new("09:00".to_string()),
                    close_time: RwSignal::new("17:00".to_string()),
                },
            )
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
                    open_time: if status == DayStatus::Open {
                        Some(s.open_time.get())
                    } else {
                        None
                    },
                    close_time: if status == DayStatus::Open {
                        Some(s.close_time.get())
                    } else {
                        None
                    },
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

                            // Location / Lat / Lng 两列对半
                            <div class="grid grid-cols-2 gap-4">
                                // 左侧：Location + Lookup
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
                                // 右侧：Latitude + Longitude
                                <div class="grid grid-cols-2 gap-3">
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

                                // ── 左右布局：左侧 Dropzone，右侧预览 ────────────
                                <div class="flex gap-4 items-start">

                                // ── 左侧：Dropzone 区域 ────────────────────────
                                <div
                                    class=move || format!(
                                        "shrink-0 w-48 h-48 border-2 border-dashed rounded-lg \
                                         text-center cursor-pointer transition-colors select-none \
                                         flex items-center justify-center {}",
                                        if drag_over.get() {
                                            "border-primary bg-primary/5"
                                        } else {
                                            "border-muted-foreground/30 \
                                             hover:border-primary/50 hover:bg-muted/30"
                                        }
                                    )
                                    // 点击 Dropzone → 触发隐藏 file input
                                    on:click=move |_| {
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
                                        on:change=move |_e| {
                                            // event_target::<HtmlInputElement> 仅 WASM 可用
                                            #[cfg(target_arch = "wasm32")]
                                            {
                                                let input: web_sys::HtmlInputElement =
                                                    event_target(&_e);
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
                                            "Drop images here"
                                        </p>
                                        <p class="text-xs opacity-70">"or click to select"</p>
                                    </div>
                                </div>

                                // ── 右侧：图片预览网格 ─────────────────────────
                                // 上传后实时渲染，Done 状态显示预览图，Uploading 显示 spinner
                                <Show when=move || !photos.get().is_empty()>
                                    <div class="flex flex-wrap gap-2 content-start flex-1 \
                                                min-h-0 max-h-36 overflow-y-auto">
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
                                                    <div class="relative group shrink-0 \
                                                                w-40 h-40 rounded-lg \
                                                                border bg-muted">
                                                        {move || match status.get() {

                                                            // 上传中：居中 spinner
                                                            PhotoStatus::Uploading => view! {
                                                                <div class="w-full h-full flex \
                                                                            items-center justify-center">
                                                                    <div class="animate-spin rounded-full \
                                                                                h-4 w-4 border-2 \
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

                                </div> // ── 结束左右 flex 布局 ──

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

                            // Labels 多选下拉（带搜索，absolute 定位相对触发器容器）
                            <div class="grid gap-2">
                                <Label>"Labels"</Label>

                                    // 相对定位容器：触发器 + 下拉面板都在此内
                                    <div class="relative">
                                    // 触发按钮：显示已选 label chips
                                    <div
                                        class="flex min-h-10 w-full cursor-pointer \
                                               flex-wrap items-center gap-1.5 rounded-md border \
                                               border-input bg-background px-3 py-2 text-sm \
                                               transition-colors hover:bg-muted/30"
                                        on:click=move |_| {
                                            show_dropdown.update(|v| *v = !*v);
                                        }
                                    >
                                        {move || {
                                            let label_list = labels.get()
                                                .and_then(|r| r.ok())
                                                .unwrap_or_default();
                                            let selected = selected_label_ids.get();
                                            if selected.is_empty() {
                                                return view! {
                                                    <span class="flex-1 text-muted-foreground">
                                                        "Select labels..."
                                                    </span>
                                                }.into_any();
                                            }
                                            view! {
                                                <div class="flex flex-1 flex-wrap gap-1">
                                                    {selected.into_iter().map(|id| {
                                                        let id_remove = id.clone();
                                                        let name = label_list.iter()
                                                            .find(|l| l.id.to_string() == id)
                                                            .map(|l| l.name.clone())
                                                            .unwrap_or_default();
                                                        view! {
                                                            <span class="flex items-center gap-1 rounded \
                                                                         bg-primary/10 px-1.5 py-0.5 \
                                                                         text-xs text-primary">
                                                                {name}
                                                                <button
                                                                    type="button"
                                                                    class="leading-none hover:text-destructive"
                                                                    on:click=move |e| {
                                                                        e.stop_propagation();
                                                                        selected_label_ids.update(|ids| {
                                                                            ids.retain(|i| i != &id_remove);
                                                                        });
                                                                    }
                                                                >"×"</button>
                                                            </span>
                                                        }
                                                    }).collect_view()}
                                                </div>
                                            }.into_any()
                                        }}
                                        <span class="ml-auto shrink-0 text-xs text-muted-foreground">
                                            {move || if show_dropdown.get() { "▲" } else { "▼" }}
                                        </span>
                                    </div>

                                    // 遮罩 + 下拉面板
                                    <Show when=move || show_dropdown.get()>
                                        // 透明遮罩（fixed 全屏），捕获外部点击关闭下拉
                                        <div
                                            class="fixed inset-0 z-40"
                                            on:click=move |_| {
                                                show_dropdown.set(false);
                                                search_query.set(String::new());
                                            }
                                        />
                                        // 面板：absolute，紧贴触发器下方，宽度同触发器
                                        <div
                                            class="absolute top-full left-0 right-0 z-50 mt-1 \
                                                   overflow-hidden rounded-md border \
                                                   bg-background shadow-lg"
                                            // 阻止点击冒泡到遮罩，避免面板内操作关闭下拉
                                            on:click=move |e| e.stop_propagation()
                                        >
                                            // 搜索框
                                            <div class="border-b p-2">
                                                <input
                                                    type="text"
                                                    placeholder="Search labels..."
                                                    class="w-full rounded border border-input \
                                                           bg-transparent px-2 py-1.5 text-sm \
                                                           outline-none focus:border-ring"
                                                    prop:value=move || search_query.get()
                                                    on:input=move |e| search_query.set(event_target_value(&e))
                                                />
                                            </div>
                                            // label 列表（按搜索词过滤）
                                            <div class="max-h-52 overflow-y-auto">
                                                {move || {
                                                    let query = search_query.get().to_lowercase();
                                                    let label_list = labels.get()
                                                        .and_then(|r| r.ok())
                                                        .unwrap_or_default();
                                                    let filtered: Vec<_> = label_list.into_iter()
                                                        .filter(|l| {
                                                            query.is_empty()
                                                                || l.name.to_lowercase().contains(&query)
                                                                || l.name.to_lowercase().contains(&query)
                                                        })
                                                        .collect();
                                                    if filtered.is_empty() {
                                                        return view! {
                                                            <p class="px-3 py-4 text-center text-sm \
                                                                       text-muted-foreground">
                                                                "No labels found."
                                                            </p>
                                                        }.into_any();
                                                    }
                                                    view! {
                                                        <div>
                                                            {filtered.into_iter().map(|label| {
                                                                let label_id  = label.id.to_string();
                                                                let id_class  = label_id.clone();
                                                                let id_check  = label_id.clone();
                                                                let id_toggle = label_id.clone();
                                                                let icon: LabelIcon = label.icon.parse()
                                                                    .unwrap_or(LabelIcon::Tag);
                                                                let display = label.name.clone();
                                                                view! {
                                                                    <button
                                                                        type="button"
                                                                        class="flex w-full cursor-pointer \
                                                                               items-center gap-2 px-3 py-2 \
                                                                               text-left text-sm transition-colors \
                                                                               hover:bg-muted"
                                                                        on:click=move |_| {
                                                                            selected_label_ids.update(|ids| {
                                                                                if let Some(pos) = ids.iter()
                                                                                    .position(|id| id == &id_toggle)
                                                                                {
                                                                                    ids.remove(pos);
                                                                                } else {
                                                                                    ids.push(id_toggle.clone());
                                                                                }
                                                                            });
                                                                        }
                                                                    >
                                                                        // 复选框
                                                                        <span class=move || {
                                                                            if selected_label_ids.get()
                                                                                .contains(&id_class)
                                                                            {
                                                                                "flex size-4 shrink-0 items-center \
                                                                                 justify-center rounded border-2 \
                                                                                 border-primary bg-primary"
                                                                            } else {
                                                                                "flex size-4 shrink-0 items-center \
                                                                                 justify-center rounded border-2 \
                                                                                 border-input"
                                                                            }
                                                                        }>
                                                                            <Show when=move || {
                                                                                selected_label_ids.get()
                                                                                    .contains(&id_check)
                                                                            }>
                                                                                <svg
                                                                                    class="size-2.5 \
                                                                                           text-primary-foreground"
                                                                                    viewBox="0 0 12 12"
                                                                                    fill="none"
                                                                                >
                                                                                    <path
                                                                                        d="M2 6l3 3 5-5"
                                                                                        stroke="currentColor"
                                                                                        stroke-width="1.5"
                                                                                        stroke-linecap="round"
                                                                                        stroke-linejoin="round"
                                                                                    />
                                                                                </svg>
                                                                            </Show>
                                                                        </span>
                                                                        // 图标
                                                                        <span class="flex size-4 shrink-0 \
                                                                                     items-center justify-center \
                                                                                     text-muted-foreground">
                                                                            {render_icon(icon)}
                                                                        </span>
                                                                        {display}
                                                                    </button>
                                                                }
                                                            }).collect_view()}
                                                        </div>
                                                    }.into_any()
                                                }}
                                            </div>
                                        </div>
                                    </Show>
                                    </div> // end relative wrapper

                                // 每个选中 ID 对应一个隐藏字段，随 ActionForm 提交
                                {move || selected_label_ids.get().into_iter().map(|id| {
                                    view! { <input type="hidden" name="label_ids" value=id /> }
                                }).collect_view()}
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

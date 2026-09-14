use explonz_shared::common::dto::{LabelDto, OpeningHourDto, SpotDto};
use icons::{CloudUpload, Pencil, Plus, Trash2};
use leptos::prelude::*;

use crate::components::ui::button::{Button, ButtonSize, ButtonVariant};

use crate::components::ui::input::{Input, InputType};
use crate::components::ui::label::Label;
use crate::components::ui::label_icon::LabelIconView;
use crate::components::ui::sheet::{
    Sheet, SheetBody, SheetClose, SheetContent, SheetContext, SheetDirection, SheetFooter,
    SheetHeader, SheetTitle, SheetTrigger,
};
use crate::server::labels::get_labels;
use crate::server::spots::{geocode_location, CreateSpot};
use explonz_shared::icons::LabelIcon;

const TEXTAREA_CLASS: &str = "text-foreground placeholder:text-muted-foreground border-input \
    flex w-full rounded-md border bg-transparent px-3 py-2 text-sm shadow-xs outline-none \
    focus-visible:border-ring focus-visible:ring-ring/50 focus-visible:ring-2 \
    dark:bg-input/30 resize-none";

#[derive(Clone, PartialEq)]
enum DayStatus {
    Open,
    Closed,
    Open24h,
}

#[derive(Clone)]
struct DaySchedule {
    status: RwSignal<DayStatus>,
    open_time: RwSignal<String>,
    close_time: RwSignal<String>,
}

#[derive(Clone)]
struct PhotoItem {
    id: u32,
    status: RwSignal<PhotoStatus>,
}

#[derive(Clone)]
enum PhotoStatus {
    Uploading,
    Done { img_id: String, url: String },
    Failed(String),
}

#[cfg(target_arch = "wasm32")]
fn upload_file(file: web_sys::File, status: RwSignal<PhotoStatus>) {
    use crate::server::spots::upload_photo;
    use server_fn::codec::MultipartData;

    leptos::task::spawn_local(async move {
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
            upload_file(file, status);
        }
    }
}

// ── Public sheet component ───────────────────────────────────────────────────

#[component]
pub fn SpotAddition(spot_dto: Option<SpotDto>, on_created: Callback<()>) -> impl IntoView {
    let is_new = spot_dto.is_none();
    view! {
        <Sheet>

            <SheetTrigger
                class=if is_new {
                    ""
                } else {
                    "border-0 shadow-none bg-transparent size-3.5"
                }
            >
                {
                    if is_new {
                        view! {
                            <Plus class="size-4" />
                            "New Spot"
                        }.into_any()
                    } else {
                        view! {
                            <Button
                                variant=ButtonVariant::Ghost
                                size=ButtonSize::IconSm
                            >
                                <Pencil class="size-3.5" />
                            </Button>
                        }.into_any()
                    }
                }

            </SheetTrigger>

            <SheetContent direction=SheetDirection::Right class="w-[640px]">
                <SpotAdditionInner spot_dto=spot_dto on_created=on_created />
            </SheetContent>
        </Sheet>
    }
}

// ── Inner form component (needs SheetContext, so lives inside Sheet) ─────────
#[component]
fn SpotAdditionInner(spot_dto: Option<SpotDto>, on_created: Callback<()>) -> impl IntoView {
    let ctx = expect_context::<SheetContext>();
    let sheet_id = ctx.target_id.clone();

    let is_edit = spot_dto.is_some();
    let s = spot_dto.unwrap_or_default();

    // NodeRef for programmatic close after successful creation
    let close_btn_ref = NodeRef::<leptos::html::Button>::new();

    let create_action = ServerAction::<CreateSpot>::new();

    // ── 基础字段 signal（用 spot 数据初始化）─────────────────────────────────
    let name_val = RwSignal::new(s.name.clone());
    let description_val = RwSignal::new(s.description.clone());
    let phone_val = RwSignal::new(s.phone.clone().unwrap_or_default());
    let website_val = RwSignal::new(s.website.clone().unwrap_or_default());

    // ── Labels ────────────────────────────────────────────────────────────────
    // show_dropdown 先于 Resource 定义，供懒加载 Effect 订阅
    let show_dropdown = RwSignal::new(false);
    let search_query: RwSignal<String> = RwSignal::new(String::new());

    // 懒加载：仅当用户打开 dropdown 时才发起 get_labels() 请求，避免每个 SpotAdditionInner
    // 挂载时都请求一次（编辑按钮每行一个，会导致 N+1 个并发请求）
    let label_fetch_trigger = RwSignal::new(false);
    Effect::new(move |_| {
        if show_dropdown.get() {
            label_fetch_trigger.set(true);
        }
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
    // 将 Resource 数据同步到普通 Signal，避免在 view 闭包中直接读取 Resource
    // （直接读取会导致 pending 状态传播到父级 Suspense，触发全屏 "load..."）
    let label_list_sig: RwSignal<Vec<LabelDto>> = RwSignal::new(vec![]);
    Effect::new(move |_| {
        if let Some(Ok(data)) = labels.get() {
            label_list_sig.set(data);
        }
    });
    // 已选 labels 存储完整 LabelDto（含 name），编辑模式直接从 spot_dto 初始化，
    // 避免为显示 chip 名称而额外依赖 label_list_sig
    let selected_labels: RwSignal<Vec<LabelDto>> = RwSignal::new(s.labels.clone());

    // ── Photos（编辑模式：已有图片作为 Done 项预填）──────────────────────────
    let initial_photos: Vec<PhotoItem> = s
        .photo_urls
        .iter()
        .enumerate()
        .map(|(i, url)| {
            let img_id = url.rsplit('/').next().unwrap_or("").to_string();
            PhotoItem {
                id: i as u32,
                status: RwSignal::new(PhotoStatus::Done { img_id, url: url.clone() }),
            }
        })
        .collect();
    let next_id = RwSignal::new(initial_photos.len() as u32);
    let photos: RwSignal<Vec<PhotoItem>> = RwSignal::new(initial_photos);
    let drag_over = RwSignal::new(false);
    let file_input_ref = NodeRef::<leptos::html::Input>::new();

    // ── Location / geocode ────────────────────────────────────────────────────
    let location_val = RwSignal::new(s.location.clone());
    let lat = RwSignal::new(if s.latitude != 0.0 { s.latitude.to_string() } else { String::new() });
    let lng = RwSignal::new(if s.longitude != 0.0 { s.longitude.to_string() } else { String::new() });

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

    // ── Opening hours（编辑模式：从 spot.opening_hours 初始化）───────────────
    const DAY_NAMES: [&str; 7] =
        ["Sunday", "Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday"];
    let schedules: Vec<(&'static str, DaySchedule)> = (0usize..7)
        .map(|i| {
            let name = DAY_NAMES[i];
            let sched = if let Some(h) = s.opening_hours.iter().find(|h| h.day_of_week == i as i16) {
                let status = if h.is_closed {
                    DayStatus::Closed
                } else if h.is_open_24h {
                    DayStatus::Open24h
                } else {
                    DayStatus::Open
                };
                DaySchedule {
                    status: RwSignal::new(status),
                    open_time: RwSignal::new(
                        h.open_time.clone().unwrap_or_else(|| "09:00".to_string()),
                    ),
                    close_time: RwSignal::new(
                        h.close_time.clone().unwrap_or_else(|| "17:00".to_string()),
                    ),
                }
            } else {
                // 无数据时的默认值：周一~五 Open，周六日 Closed
                let default_status = if i == 0 || i == 6 { DayStatus::Closed } else { DayStatus::Open };
                DaySchedule {
                    status: RwSignal::new(default_status),
                    open_time: RwSignal::new("09:00".to_string()),
                    close_time: RwSignal::new("17:00".to_string()),
                }
            };
            (name, sched)
        })
        .collect();
    let schedules = StoredValue::new(schedules);

    let hours_memo = Memo::new(move |_| {
        let entries: Vec<OpeningHourDto> = schedules
            .get_value()
            .iter()
            .enumerate()
            .map(|(i, (_, s))| {
                let status = s.status.get();
                OpeningHourDto {
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

    // ── On success: call parent callback + close sheet ────────────────────────
    Effect::new(move |_| {
        if let Some(Ok(_)) = create_action.value().get() {
            on_created.run(());
            #[cfg(target_arch = "wasm32")]
            if let Some(btn) = close_btn_ref.get() {
                let _ = btn.click();
            }
        }
    });

    view! {
        // Hidden button used to close the sheet programmatically after creation
        <button
            type="button"
            node_ref=close_btn_ref
            data-sheet-close=sheet_id
            class="hidden"
        />

        // ==============================
        // Header
        // ==============================
        <SheetHeader>
            <SheetTitle>{if is_edit { "Edit Spot" } else { "New Spot" }}</SheetTitle>
        </SheetHeader>

        // ==============================
        // Form (wraps Body + Footer so hidden inputs are included in submission)
        // ==============================
        <ActionForm action=create_action>

            <SheetBody>
                <div class="flex flex-col gap-5">

                    // Name
                    <div class="grid gap-2">
                        <Label html_for="name">"Name"</Label>
                        <Input id="name" name="name" placeholder="e.g. Kumeu Orchard"
                            required=true bind_value=name_val />
                    </div>

                    // Location + Lat/Lng
                    <div class="grid grid-cols-2 gap-4">
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
                                    attr:r#type="button"
                                    attr:disabled=move || geocode_action.pending().get()
                                    on:click=move |_| {
                                        geocode_action.dispatch(location_val.get_untracked());
                                    }
                                >
                                    {move || if geocode_action.pending().get() { "Looking up..." } else { "Lookup" }}
                                </Button>
                            </div>
                            {move || geocode_action.value().get()
                                .and_then(|r| r.err())
                                .map(|e| view! {
                                    <p class="text-xs text-destructive">{e.to_string()}</p>
                                })}
                        </div>
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

                    // Description
                    <div class="grid gap-2">
                        <Label html_for="description">"Description"</Label>
                        <textarea id="description" name="description" rows="3"
                            placeholder="Describe this spot..." class=TEXTAREA_CLASS
                            prop:value=move || description_val.get()
                            on:input=move |e| description_val.set(event_target_value(&e))
                        />
                    </div>

                    // Photos
                    <div class="grid gap-3">
                        <Label>"Photos"</Label>
                        <p class="text-xs text-muted-foreground">
                            "First image will be used as cover · Supports JPG, PNG, WebP"
                        </p>
                        <div class="flex gap-4 items-start">
                            // Dropzone
                            <div
                                class=move || format!(
                                    "shrink-0 w-36 h-36 border-2 border-dashed rounded-lg \
                                     text-center cursor-pointer transition-colors select-none \
                                     flex items-center justify-center {}",
                                    if drag_over.get() {
                                        "border-primary bg-primary/5"
                                    } else {
                                        "border-muted-foreground/30 hover:border-primary/50 hover:bg-muted/30"
                                    }
                                )
                                on:click=move |_| {
                                    #[cfg(target_arch = "wasm32")]
                                    if let Some(input) = file_input_ref.get() { input.click(); }
                                }
                                on:dragover=move |e| { e.prevent_default(); drag_over.set(true); }
                                on:dragleave=move |_| drag_over.set(false)
                                on:drop=move |e| {
                                    e.prevent_default();
                                    drag_over.set(false);
                                    #[cfg(target_arch = "wasm32")]
                                    if let Some(dt) = e.data_transfer() {
                                        if let Some(files) = dt.files() {
                                            process_files(files, next_id, photos);
                                        }
                                    }
                                }
                            >
                                <input
                                    type="file"
                                    accept="image/*"
                                    multiple=true
                                    class="hidden"
                                    node_ref=file_input_ref
                                    on:change=move |_e| {
                                        #[cfg(target_arch = "wasm32")]
                                        {
                                            let input: web_sys::HtmlInputElement = event_target(&_e);
                                            if let Some(files) = input.files() {
                                                process_files(files, next_id, photos);
                                            }
                                        }
                                    }
                                />
                                <div class="flex flex-col items-center gap-1.5 \
                                            text-muted-foreground pointer-events-none">
                                    <CloudUpload class="size-6 opacity-50" />
                                    <p class="text-xs font-medium">"Drop images"</p>
                                    <p class="text-xs opacity-70">"or click"</p>
                                </div>
                            </div>

                            // Preview grid
                            <Show when=move || !photos.get().is_empty()>
                                <div class="flex flex-wrap gap-2 content-start flex-1 \
                                            min-h-0 max-h-36 overflow-y-auto">
                                    <For
                                        each=move || photos.get()
                                        key=|p| p.id
                                        children=move |item| {
                                            let status = item.status;
                                            let local_id = item.id;
                                            let is_cover = move || {
                                                photos.get().first().map(|p| p.id) == Some(local_id)
                                            };
                                            view! {
                                                <div class="relative group shrink-0 w-32 h-32 \
                                                            rounded-lg border bg-muted">
                                                    {move || match status.get() {
                                                        PhotoStatus::Uploading => view! {
                                                            <div class="w-full h-full flex \
                                                                        items-center justify-center">
                                                                <div class="animate-spin rounded-full \
                                                                            h-4 w-4 border-2 border-primary \
                                                                            border-t-transparent" />
                                                            </div>
                                                        }.into_any(),
                                                        PhotoStatus::Done { url, img_id } => view! {
                                                            <img src=url.clone()
                                                                class="w-full h-full object-cover rounded-lg" />
                                                            <Show when=is_cover>
                                                                <span class="absolute top-1 left-1 text-xs \
                                                                             bg-primary text-primary-foreground \
                                                                             px-1.5 py-0.5 rounded font-medium \
                                                                             pointer-events-none">
                                                                    "Cover"
                                                                </span>
                                                            </Show>
                                                            <button
                                                                type="button"
                                                                class="absolute top-1 right-1 bg-destructive \
                                                                       text-destructive-foreground rounded p-1 \
                                                                       opacity-0 group-hover:opacity-100 \
                                                                       transition-opacity"
                                                                on:click=move |e| {
                                                                    e.stop_propagation();
                                                                    let id = img_id.clone();
                                                                    leptos::task::spawn_local(async move {
                                                                        use crate::server::spots::delete_photo;
                                                                        let _ = delete_photo(id).await;
                                                                        photos.update(|v| v.retain(|p| p.id != local_id));
                                                                    });
                                                                }
                                                            >
                                                                <Trash2 class="size-3" />
                                                            </button>
                                                        }.into_any(),
                                                        PhotoStatus::Failed(err) => view! {
                                                            <div class="w-full h-full flex flex-col \
                                                                        items-center justify-center gap-1 p-2">
                                                                <p class="text-xs text-destructive \
                                                                          text-center line-clamp-3">{err}</p>
                                                            </div>
                                                        }.into_any(),
                                                    }}
                                                </div>
                                            }
                                        }
                                    />
                                </div>
                            </Show>
                        </div>

                        // Hidden photo URL fields
                        {move || photos.get().into_iter()
                            .filter_map(|p| {
                                if let PhotoStatus::Done { url, .. } = p.status.get() {
                                    Some(view! { <input type="hidden" name="photo_urls[]" value=url /> })
                                } else { None }
                            })
                            .collect_view()}
                    </div>

                    // Labels
                    <div class="grid gap-2">
                        <Label>"Labels"</Label>
                        <div class="relative">
                            <div
                                class="flex min-h-10 w-full cursor-pointer flex-wrap items-center \
                                       gap-1.5 rounded-md border border-input bg-background px-3 \
                                       py-2 text-sm transition-colors hover:bg-muted/30"
                                on:click=move |_| show_dropdown.update(|v| *v = !*v)
                            >
                                {move || {
                                    let selected = selected_labels.get();
                                    if selected.is_empty() {
                                        return view! {
                                            <span class="flex-1 text-muted-foreground">"Select labels..."</span>
                                        }.into_any();
                                    }
                                    view! {
                                        <div class="flex flex-1 flex-wrap gap-1">
                                            {selected.into_iter().map(|label| {
                                                let label_id = label.id;
                                                let name = label.name.clone();
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
                                                                selected_labels.update(|sel| {
                                                                    sel.retain(|l| l.id != label_id);
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

                            <Show when=move || show_dropdown.get()>
                                <div
                                    class="fixed inset-0 z-40"
                                    on:click=move |_| {
                                        show_dropdown.set(false);
                                        search_query.set(String::new());
                                    }
                                />
                                <div
                                    class="absolute top-full left-0 right-0 z-50 mt-1 \
                                           overflow-hidden rounded-md border bg-background shadow-lg"
                                    on:click=move |e| e.stop_propagation()
                                >
                                    <div class="border-b p-2">
                                        <input
                                            type="text"
                                            placeholder="Search labels..."
                                            class="w-full rounded border border-input bg-transparent \
                                                   px-2 py-1.5 text-sm outline-none focus:border-ring"
                                            prop:value=move || search_query.get()
                                            on:input=move |e| search_query.set(event_target_value(&e))
                                        />
                                    </div>
                                    <div class="max-h-52 overflow-y-auto">
                                        {move || {
                                            let query = search_query.get().to_lowercase();
                                            let label_list = label_list_sig.get();
                                            let filtered: Vec<_> = label_list.into_iter()
                                                .filter(|l| query.is_empty()
                                                    || l.name.to_lowercase().contains(&query))
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
                                                        let label_for_toggle = label.clone();
                                                        let label_id  = label.id.to_string();
                                                        let id_class  = label_id.clone();
                                                        let id_check  = label_id.clone();
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
                                                                    selected_labels.update(|sel| {
                                                                        if let Some(pos) = sel.iter()
                                                                            .position(|l| l.id == label_for_toggle.id)
                                                                        {
                                                                            sel.remove(pos);
                                                                        } else {
                                                                            sel.push(label_for_toggle.clone());
                                                                        }
                                                                    });
                                                                }
                                                            >
                                                                <span class=move || {
                                                                    if selected_labels.get().iter().any(|l| l.id.to_string() == id_class) {
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
                                                                        selected_labels.get().iter().any(|l| l.id.to_string() == id_check)
                                                                    }>
                                                                        <svg
                                                                            class="size-2.5 text-primary-foreground"
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
                                                                <span class="flex size-4 shrink-0 \
                                                                             items-center justify-center \
                                                                             text-muted-foreground">
                                                                    <LabelIconView icon=icon />
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
                        </div>

                        // Hidden label_id fields
                        {move || selected_labels.get().into_iter().map(|l| {
                            view! { <input type="hidden" name="label_ids[]" value=l.id.to_string() /> }
                        }).collect_view()}
                    </div>

                    // Phone & Website
                    <div class="grid grid-cols-2 gap-4">
                        <div class="grid gap-2">
                            <Label html_for="phone">"Phone (optional)"</Label>
                            <Input r#type=InputType::Tel id="phone" name="phone"
                                placeholder="+64 9 123 4567" bind_value=phone_val />
                        </div>
                        <div class="grid gap-2">
                            <Label html_for="website">"Website (optional)"</Label>
                            <Input r#type=InputType::Url id="website" name="website"
                                placeholder="https://example.com" bind_value=website_val />
                        </div>
                    </div>

                    // Opening Hours
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

                </div>
            </SheetBody>

            // ==============================
            // Footer
            // ==============================
            <SheetFooter class="flex flex-row">
                {move || create_action.value().get()
                    .and_then(|r| r.err())
                    .map(|e| view! {
                        <p class="text-sm text-destructive">{e.to_string()}</p>
                    })}

                <SheetClose variant=ButtonVariant::Outline>"Cancel"</SheetClose>

                <Button attr:disabled=move || create_action.pending().get()>
                    {move || if create_action.pending().get() {
                        if is_edit { "Saving..." } else { "Creating..." }
                    } else {
                        if is_edit { "Save Spot" } else { "Create Spot" }
                    }}
                </Button>
            </SheetFooter>

        </ActionForm>
    }
}

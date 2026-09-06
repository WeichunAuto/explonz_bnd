use explonz_shared::common::dto::LabelDto;
use icons::{Pencil, Plus, Tag, Trash2, X};
use leptos::prelude::*;

use crate::components::ui::button::{Button, ButtonSize, ButtonVariant};
use crate::components::ui::card::{Card, CardContent};
use crate::components::ui::input::{Input, InputType};
use crate::components::ui::label::Label;
use crate::server::labels::{get_labels, CreateLabel, DeleteLabel, UpdateLabel};
use explonz_shared::icons::LabelIcon;
use strum::IntoEnumIterator;

// ---------------------------------------------------------------------------
// ---------------------------------------------------------------------------
// 图标渲染分发（icons crate 组件无法在运行时动态构造，故用 match 静态分发）
// ---------------------------------------------------------------------------

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
        _ => view! { <Tag /> }.into_any(), // 默认 Tag
    }
}

// ---------------------------------------------------------------------------
// 页面组件
// ---------------------------------------------------------------------------

#[component]
pub fn LabelList() -> impl IntoView {
    // ── 数据 ─────────────────────────────────────────────────────────────
    // SSR 阶段 Effect 不会运行，fetch_trigger 保持 false，Resource 跳过实际请求。
    // 客户端 mount 后 Effect 触发，source 变为 true，Resource 重新从后端 API 加载数据。
    let fetch_trigger = RwSignal::new(false);
    Effect::new(move |_| {
        fetch_trigger.set(true);
    });

    let labels = Resource::new(
        move || fetch_trigger.get(),
        move |ready| async move {
            if !ready {
                return Ok(vec![]);
            }
            get_labels().await
        },
    );

    // ── 表单状态 ──────────────────────────────────────────────────────────
    let show_form: RwSignal<bool> = RwSignal::new(false);
    let edit_id: RwSignal<Option<String>> = RwSignal::new(None);
    let form_name: RwSignal<String> = RwSignal::new(String::new());
    let form_desc: RwSignal<String> = RwSignal::new(String::new());
    let form_icon: RwSignal<LabelIcon> = RwSignal::new(LabelIcon::Tag);
    let show_icon_picker: RwSignal<bool> = RwSignal::new(false);

    // ── 删除二次确认状态 ───────────────────────────────────────────────────
    let deleting_id: RwSignal<Option<String>> = RwSignal::new(None);

    // ── Server Actions ────────────────────────────────────────────────────
    let create_action = ServerAction::<CreateLabel>::new();
    let update_action = ServerAction::<UpdateLabel>::new();
    let delete_action = ServerAction::<DeleteLabel>::new();

    // ── 新建 成功后重新拉取列表 & 重置表单 ────────────────────────────────────
    Effect::new(move |_| {
        if matches!(create_action.value().get(), Some(Ok(_))) {
            labels.refetch();
            show_form.set(false);
            show_icon_picker.set(false);
            edit_id.set(None);
            form_name.set(String::new());
            form_desc.set(String::new());
            form_icon.set(LabelIcon::Tag);
        }
    });

    // ── 更新 成功后重新拉取列表 & 重置表单 ────────────────────────────────────
    Effect::new(move |_| {
        if matches!(update_action.value().get(), Some(Ok(_))) {
            labels.refetch();
            show_form.set(false);
            show_icon_picker.set(false);
            edit_id.set(None);
            form_name.set(String::new());
            form_desc.set(String::new());
            form_icon.set(LabelIcon::Tag);
        }
    });

    // ── 删除 成功后重新拉取列表 & 重置表单 ────────────────────────────────────
    Effect::new(move |_| {
        if matches!(delete_action.value().get(), Some(Ok(_))) {
            labels.refetch();
            deleting_id.set(None);
        }
    });

    // ── 提交处理 ──────────────────────────────────────────────────────────
    let on_submit = move |_| {
        let name = form_name.get_untracked();
        let description = form_desc.get_untracked();
        let icon = form_icon.get_untracked();
        if name.trim().is_empty() || description.trim().is_empty() {
            return;
        }
        match edit_id.get_untracked() {
            Some(id) => update_action.dispatch(UpdateLabel {
                id,
                name,
                description,
                icon: icon.to_string(),
            }),
            None => create_action.dispatch(CreateLabel {
                name,
                description,
                icon: icon.to_string(),
            }),
        };
    };

    view! {
        <div class="p-6 max-w-6xl mx-auto flex flex-col gap-6">

            // ── 页面标题 + 新建按钮 ───────────────────────────────────────
            <div class="flex items-center justify-between">
                <h1 class="text-2xl font-bold tracking-tight">"Labels"</h1>
                <Button
                    variant=ButtonVariant::Default
                    on:click=move |_| {
                        // 新建：清空表单，打开面板
                        edit_id.set(None);
                        form_name.set(String::new());
                        form_desc.set(String::new());
                        form_icon.set(LabelIcon::Tag);
                        show_icon_picker.set(false);
                        show_form.set(true);
                    }
                >
                    <Plus class="size-4" />
                    "New Label"
                </Button>
            </div>

            // ── 创建 / 编辑面板（右侧滑入 overlay）────────────────────────
            <Show when=move || show_form.get()>
                // 半透明背景遮罩，点击关闭
                <div
                    class="fixed inset-0 bg-black/40 z-40"
                    on:click=move |_| {
                        show_form.set(false);
                        show_icon_picker.set(false);
                    }
                />

                // 右侧面板
                <div class="fixed right-0 top-0 h-full w-[440px] bg-background border-l z-50 \
                             shadow-xl overflow-y-auto flex flex-col">

                    // 面板头
                    <div class="flex items-center justify-between px-6 py-4 border-b shrink-0">
                        <h2 class="text-lg font-semibold">
                            {move || if edit_id.get().is_some() { "Edit Label" } else { "New Label" }}
                        </h2>
                        <Button
                            variant=ButtonVariant::Ghost
                            size=ButtonSize::Icon
                            on:click=move |_| {
                                show_form.set(false);
                                show_icon_picker.set(false);
                            }
                        >
                            <X class="size-4" />
                        </Button>
                    </div>

                    // 面板主体
                    <div class="flex flex-col gap-5 px-6 py-5 flex-1">

                        // ── 图标选择器 ────────────────────────────────────
                        <div class="grid gap-2">
                            <Label>"Icon"</Label>

                            // 当前图标触发按钮
                            <button
                                type="button"
                                class="flex items-center gap-2 border rounded-md px-3 py-2 text-sm \
                                       bg-background w-full text-left hover:bg-accent \
                                       transition-colors cursor-pointer"
                                on:click=move |_| show_icon_picker.update(|v| *v = !*v)
                            >
                                <span class="size-4 flex items-center justify-center text-foreground">
                                    {move || render_icon(form_icon.get())}
                                </span>
                                <span class="flex-1">{move || form_icon.get().to_string()}</span>
                                // <span class="flex-1">"T"</span>
                                <span class="text-muted-foreground text-xs">
                                    {move || if show_icon_picker.get() { "▲" } else { "▼" }}
                                </span>
                            </button>

                            // 图标网格（展开时显示）
                            <Show when=move || show_icon_picker.get()>
                                <div class="border rounded-md p-2 grid grid-cols-5 gap-1 bg-muted/20">
                                    {
                                        LabelIcon::iter().map(|icon| {
                                        view! {
                                            <button
                                                type="button"
                                                title=icon.to_string()
                                                class=move || {
                                                    let selected = form_icon.get() == icon;
                                                    if selected {
                                                        "flex flex-col items-center gap-1 p-2 rounded text-xs \
                                                         bg-primary text-primary-foreground cursor-pointer"
                                                    } else {
                                                        "flex flex-col items-center gap-1 p-2 rounded text-xs \
                                                         hover:bg-accent transition-colors cursor-pointer"
                                                    }
                                                }
                                                on:click=move |_| {
                                                    form_icon.set(icon);
                                                    show_icon_picker.set(false);
                                                }
                                            >
                                                <span class="size-4 flex items-center justify-center">
                                                    {render_icon(icon)}
                                                </span>
                                                <span class="text-[10px] truncate w-full text-center leading-none">
                                                    {icon.to_string()}
                                                </span>
                                            </button>
                                        }
                                    }).collect_view()}
                                </div>
                            </Show>
                        </div>

                        // ── Name（英文标识符） ─────────────────────────────
                        <div class="grid gap-2">
                            <Label html_for="label_name">"Name"</Label>
                            <Input
                                r#type=InputType::Text
                                id="label_name"
                                placeholder="e.g. family_friendly"
                                bind_value=form_name
                            />
                            <p class="text-xs text-muted-foreground">
                                "Lowercase letters, numbers, and underscores only."
                            </p>
                        </div>

                        // ── Description（可读说明） ────────────────────────
                        <div class="grid gap-2">
                            <Label html_for="label_desc">"Description"</Label>
                            <Input
                                r#type=InputType::Text
                                id="label_desc"
                                placeholder="e.g. Family Friendly"
                                bind_value=form_desc
                            />
                        </div>

                        // ── 错误提示 ──────────────────────────────────────
                        {move || {
                            let err = create_action.value().get()
                                .and_then(|r| r.err())
                                .map(|e| e.to_string())
                                .or_else(|| {
                                    update_action.value().get()
                                        .and_then(|r| r.err())
                                        .map(|e| e.to_string())
                                });
                            err.map(|msg| view! {
                                <p class="text-sm text-destructive">{msg}</p>
                            })
                        }}
                    </div>

                    // 面板底部按钮
                    <div class="flex gap-3 px-6 py-4 border-t shrink-0">
                        <Button
                            variant=ButtonVariant::Outline
                            class="flex-1"
                            on:click=move |_| {
                                show_form.set(false);
                                show_icon_picker.set(false);
                            }
                        >
                            "Cancel"
                        </Button>
                        <Button
                            variant=ButtonVariant::Default
                            class="flex-1"
                            on:click=on_submit
                        >
                            {move || {
                                if create_action.pending().get() || update_action.pending().get() {
                                    "Saving..."
                                } else {
                                    "Save"
                                }
                            }}
                        </Button>
                    </div>
                </div>
            </Show>

            // ── 标签列表表格 ───────────────────────────────────────────────
            <Card>
                <CardContent class="p-0 overflow-hidden">
                    <Suspense fallback=|| view! {
                        <div class="px-6 py-10 text-center text-muted-foreground text-sm">
                            "Loading..."
                        </div>
                    }>
                        {move || {
                            let result = labels.get();

                            // 仍在加载中：交给 Suspense fallback 处理
                            let Some(result) = result else {
                                return view! { <div /> }.into_any();
                            };

                            // 加载失败：显示错误信息
                            let list = match result {
                                Err(e) => return view! {
                                    <div class="px-6 py-10 text-center text-destructive text-sm">
                                        {format!("Failed to load labels: {e}")}
                                    </div>
                                }.into_any(),
                                Ok(v) => v,
                            };

                            if list.is_empty() {
                                return view! {
                                    <div class="px-6 py-10 text-center text-muted-foreground text-sm">
                                        "No labels yet. Click \"New Label\" to create one."
                                    </div>
                                }.into_any();
                            }

                            view! {
                                <table class="w-full text-sm">
                                    <thead class="bg-muted/60 text-muted-foreground border-b">
                                        <tr>
                                            <th class="px-4 py-3 text-left font-medium w-14">
                                                "Icon"
                                            </th>
                                            <th class="px-4 py-3 text-left font-medium w-48">
                                                "Name"
                                            </th>
                                            <th class="px-4 py-3 text-left font-medium">
                                                "Description"
                                            </th>
                                            <th class="px-4 py-3 text-right font-medium w-28">
                                                "Actions"
                                            </th>
                                        </tr>
                                    </thead>
                                    <tbody>
                                        {list.into_iter().map(|label: LabelDto| {
                                            let icon_name   = label.icon.clone();
                                            let label_id = label.id.to_string();
                                            // 每个 handler 需要独立 clone，避免 move 冲突
                                            let id_edit     = label_id.clone();
                                            let id_confirm  = label_id.clone();
                                            let id_delete   = label_id.clone();
                                            let id_cancel   = label_id.clone();

                                            view! {
                                                <tr class="border-b last:border-0 hover:bg-muted/30 \
                                                           transition-colors">

                                                    // Icon 列
                                                    <td class="px-4 py-3">
                                                        <span class="size-5 flex items-center \
                                                                     text-muted-foreground">
                                                            {render_icon(icon_name.parse().unwrap())}
                                                        </span>
                                                    </td>

                                                    // Name 列（等宽字体，更易识别 identifier）
                                                    <td class="px-4 py-3 font-mono text-xs \
                                                               text-muted-foreground">
                                                        {label.name.clone()}
                                                    </td>

                                                    // Description 列
                                                    <td class="px-4 py-3">
                                                        {label.description.clone()}
                                                    </td>

                                                    // Actions 列
                                                    <td class="px-4 py-3">
                                                        <div class="flex items-center justify-end gap-1">
                                                            // 编辑按钮
                                                            <Button
                                                                variant=ButtonVariant::Ghost
                                                                size=ButtonSize::IconSm
                                                                on:click=move |_| {
                                                                    // 点击时从 resource 现取最新数据，避免闭包捕获旧快照
                                                                    let Some(Ok(list)) = labels.get_untracked() else { return };
                                                                    let Some(label) = list.into_iter().find(|l| l.id.to_string() == id_edit) else { return };
                                                                    edit_id.set(Some(id_edit.clone()));
                                                                    form_name.set(label.name.clone());
                                                                    form_desc.set(label.description.clone());
                                                                    form_icon.set(label.icon.parse().unwrap_or(LabelIcon::Tag));
                                                                    show_icon_picker.set(false);
                                                                    show_form.set(true);
                                                                }
                                                            >
                                                                <Pencil class="size-3.5" />
                                                            </Button>

                                                            // 删除：未确认时显示垃圾桶，确认时显示 Delete/Cancel
                                                            <Show
                                                                when=move || {
                                                                    deleting_id.get().as_deref()
                                                                        == Some(id_confirm.as_str())
                                                                }
                                                                fallback=move || {
                                                                    let idc = id_cancel.clone();

                                                                    view! {
                                                                        <Button
                                                                            variant=ButtonVariant::Ghost
                                                                            size=ButtonSize::IconSm
                                                                            on:click=move |_| {
                                                                                deleting_id.set(Some(idc.clone()));
                                                                            }
                                                                        >
                                                                            <Trash2 class="size-3.5" />
                                                                        </Button>
                                                                    }
                                                                }
                                                            >
                                                                <div class="flex items-center gap-1">
                                                                    <Button
                                                                        variant=ButtonVariant::Destructive
                                                                        size=ButtonSize::Sm
                                                                        on:click={
                                                                            let id = id_delete.clone();
                                                                            move |_| {
                                                                                delete_action.dispatch(
                                                                                    DeleteLabel { id: id.clone() }
                                                                                );
                                                                            }
                                                                        }
                                                                    >
                                                                        "Delete"
                                                                    </Button>

                                                                    <Button
                                                                        variant=ButtonVariant::Ghost
                                                                        size=ButtonSize::IconSm
                                                                        on:click=move |_| {
                                                                            deleting_id.set(None)
                                                                        }
                                                                    >
                                                                        <X class="size-3.5" />
                                                                    </Button>
                                                                </div>
                                                            </Show>
                                                        </div>
                                                    </td>
                                                </tr>
                                            }
                                        }).collect_view()}
                                    </tbody>
                                </table>
                            }.into_any()
                        }}
                    </Suspense>
                </CardContent>
            </Card>

            // ── 删除 Action 错误提示 ───────────────────────────────────────
            {move || {
                delete_action.value().get()
                    .and_then(|r| r.err())
                    .map(|e| view! {
                        <p class="text-sm text-destructive">{e.to_string()}</p>
                    })
            }}
        </div>
    }
}

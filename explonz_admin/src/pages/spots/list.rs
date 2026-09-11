use explonz_shared::common::{dto::SpotDto, pagination::Page};
use icons::{Pencil, Plus, Trash2, X};
use leptos::prelude::*;
use leptos_router::hooks::{use_location, use_navigate};

use crate::components::ui::button::{Button, ButtonSize, ButtonVariant};
use crate::components::ui::card::{Card, CardContent};
use crate::components::ui::input::{Input, InputType};
use crate::components::ui::label::Label;
use crate::components::ui::table::{Table, TableBody, TableCell, TableHead, TableHeader, TableRow};
use crate::server::spots::{delete_spot, get_spots, DeleteSpot};

const PAGE_SIZE: u64 = 2;

#[component]
pub fn SpotList() -> impl IntoView {
    // ── 导航 ─────────────────────────────────────────────────────────────
    let location = use_location();
    let navigate = use_navigate();

    // 所有页面跳转通过此信号驱动，navigate 只在一个 Effect 内调用，避免多处 move
    let nav_target: RwSignal<Option<String>> = RwSignal::new(None);
    Effect::new(move |_| {
        if let Some(target) = nav_target.get() {
            navigate(&target, Default::default());
            nav_target.set(None);
        }
    });

    // 当前路由父路径（组件挂载时计算一次）
    let spots_base = {
        let current = location.pathname.get_untracked();
        current
            .rsplit_once('/')
            .map(|(b, _)| b.to_string())
            .unwrap_or_else(|| "/".to_string())
    };

    // ── 搜索输入（即时绑定，不触发请求）──────────────────────────────────
    let input_id: RwSignal<String> = RwSignal::new(String::new());
    let input_name: RwSignal<String> = RwSignal::new(String::new());

    // ── 已提交的查询条件（点击 Search 后才写入，才触发请求）──────────────
    let query_id: RwSignal<Option<String>> = RwSignal::new(None);
    let query_name: RwSignal<Option<String>> = RwSignal::new(None);
    let current_page: RwSignal<u64> = RwSignal::new(1);

    // ── 数据加载 ──────────────────────────────────────────────────────────
    // SSR 阶段 Effect 不运行，fetch_trigger 为 false，Resource 返回空数据。
    // 客户端挂载后 Effect 触发，source 变为 true，Resource 向后端发起请求。
    let fetch_trigger = RwSignal::new(false);
    Effect::new(move |_| {
        fetch_trigger.set(true);
    });

    let spots_page: Resource<Result<Page<SpotDto>, ServerFnError>> = Resource::new(
        move || {
            (
                fetch_trigger.get(),
                query_id.get(),
                query_name.get(),
                current_page.get(),
            )
        },
        move |(ready, id, name, page)| async move {
            if !ready {
                return Ok(Page {
                    data: vec![],
                    total: 0,
                    page: 1,
                    size: PAGE_SIZE,
                });
            }
            get_spots(id, name, page, PAGE_SIZE).await
        },
    );

    // ── 分页信息（信号，由 Resource 加载后更新，供分页控件响应式绑定）────
    let total_items: RwSignal<u64> = RwSignal::new(0);
    let total_pages: RwSignal<u64> = RwSignal::new(1);

    Effect::new(move |_| {
        if let Some(Ok(ref p)) = spots_page.get() {
            total_items.set(p.total);
            let tp = if p.size == 0 {
                1
            } else {
                ((p.total + p.size - 1) / p.size).max(1)
            };
            total_pages.set(tp);
        }
    });

    // ── 删除 ──────────────────────────────────────────────────────────────
    let deleting_id: RwSignal<Option<String>> = RwSignal::new(None);
    let delete_action = ServerAction::<DeleteSpot>::new();

    Effect::new(move |_| {
        if matches!(delete_action.value().get(), Some(Ok(_))) {
            spots_page.refetch();
            deleting_id.set(None);
        }
    });

    // ── 搜索提交 / 重置 ───────────────────────────────────────────────────
    let on_search = move |_| {
        let id_val = input_id.get_untracked().trim().to_string();
        let name_val = input_name.get_untracked().trim().to_string();
        query_id.set(if id_val.is_empty() {
            None
        } else {
            Some(id_val)
        });
        query_name.set(if name_val.is_empty() {
            None
        } else {
            Some(name_val)
        });
        current_page.set(1);
    };

    let on_reset = move |_| {
        input_id.set(String::new());
        input_name.set(String::new());
        query_id.set(None);
        query_name.set(None);
        current_page.set(1);
    };

    view! {
        <div class="p-6 max-w-6xl mx-auto flex flex-col gap-6">

            // ── 页头 ──────────────────────────────────────────────────────
            <div class="flex items-center justify-between">
                <h1 class="text-2xl font-bold tracking-tight">"Spots"</h1>
                <Button
                    variant=ButtonVariant::Default
                    on:click={
                        let bp = spots_base.clone();
                        move |_| nav_target.set(Some(format!("{}/addition", bp)))
                    }
                >
                    <Plus class="size-4" />
                    "New Spot"
                </Button>
            </div>

            // ── 搜索栏 ────────────────────────────────────────────────────
            <div class="flex flex-wrap items-end gap-3">
                <div class="grid gap-1.5">
                    <Label html_for="search_id">"Spot ID"</Label>
                    <Input
                        r#type=InputType::Text
                        id="search_id"
                        placeholder="UUID"
                        bind_value=input_id
                    />
                </div>
                <div class="grid gap-1.5 min-w-[200px]">
                    <Label html_for="search_name">"Name"</Label>
                    <Input
                        r#type=InputType::Text
                        id="search_name"
                        placeholder="Search by name..."
                        bind_value=input_name
                    />
                </div>
                <Button variant=ButtonVariant::Default on:click=on_search>
                    "Search"
                </Button>
                <Button variant=ButtonVariant::Outline on:click=on_reset>
                    "Reset"
                </Button>
            </div>

            // ── 列表卡片 ──────────────────────────────────────────────────
            <Card>
                <CardContent class="p-0 overflow-hidden">

                    // 表格（Suspense 包裹，加载中显示占位）
                    <Suspense fallback=|| {
                        view! {
                            <div class="px-6 py-10 text-center text-muted-foreground text-sm">
                                "Loading..."
                            </div>
                        }
                    }>
                        {move || {
                            let sb = spots_base.clone();
                            let result = spots_page.get();
                            let Some(result) = result else {
                                return view! { <div /> }.into_any();
                            };
                            let page_data = match result {
                                Err(e) => {
                                    return view! {
                                        <div class="px-6 py-10 text-center text-destructive text-sm">
                                            {format!("Failed to load spots: {e}")}
                                        </div>
                                    }
                                        .into_any()
                                }
                                Ok(v) => v,
                            };
                            if page_data.data.is_empty() {
                                return view! {
                                    <div class="px-6 py-10 text-center text-muted-foreground text-sm">
                                        "No spots found."
                                    </div>
                                }
                                    .into_any();
                            }
                            view! {
                                <Table>
                                    <TableHeader>
                                        <TableRow>
                                            <TableHead class="w-14">"Cover"</TableHead>
                                            <TableHead>"Name"</TableHead>
                                            <TableHead>"Location"</TableHead>
                                            <TableHead class="w-20">"Rating"</TableHead>
                                            <TableHead class="w-42">"Updated"</TableHead>
                                            <TableHead class="w-28 text-right">"Actions"</TableHead>
                                        </TableRow>
                                    </TableHeader>
                                    <TableBody>
                                        {page_data
                                            .data
                                            .into_iter()
                                            .map(|spot: SpotDto| {
                                                let spot_id = spot.id.to_string();
                                                let edit_url = format!("{}/edit/{}", sb, spot_id);
                                                let id_confirm = spot_id.clone();
                                                let id_delete = spot_id.clone();
                                                let id_cancel = spot_id.clone();
                                                let cover_url = spot.photo_urls.into_iter().next();
                                                let rating_str = spot.rating.to_string();
                                                let updated_str = spot
                                                    .updated_at
                                                    .format("%Y-%m-%d %H:%M")
                                                    .to_string();
                                                view! {
                                                    <TableRow>
                                                        // 封面图
                                                        <TableCell>
                                                            {if let Some(url) = cover_url {
                                                                view! {
                                                                    <img
                                                                        src=url
                                                                        class="w-10 h-10 object-cover rounded"
                                                                        alt="cover"
                                                                    />
                                                                }
                                                                    .into_any()
                                                            } else {
                                                                view! {
                                                                    <div class="w-10 h-10 bg-muted rounded" />
                                                                }
                                                                    .into_any()
                                                            }}
                                                        </TableCell>

                                                        // Name
                                                        <TableCell class="font-medium">
                                                            {spot.name}
                                                        </TableCell>

                                                        // Location
                                                        <TableCell class="text-muted-foreground max-w-[200px] truncate">
                                                            {spot.location}
                                                        </TableCell>

                                                        // Rating
                                                        <TableCell class="text-muted-foreground">
                                                            {rating_str}
                                                        </TableCell>

                                                        // Updated
                                                        <TableCell class="text-muted-foreground text-xs">
                                                            {updated_str}
                                                        </TableCell>

                                                        // Actions
                                                        <TableCell>
                                                            <div class="flex items-center justify-end gap-1">
                                                                // 编辑
                                                                <Button
                                                                    variant=ButtonVariant::Ghost
                                                                    size=ButtonSize::IconSm
                                                                    on:click={
                                                                        let url = edit_url.clone();
                                                                        move |_| nav_target
                                                                            .set(Some(url.clone()))
                                                                    }
                                                                >
                                                                    <Pencil class="size-3.5" />
                                                                </Button>

                                                                // 删除二次确认
                                                                <Show
                                                                    when=move || {
                                                                        deleting_id
                                                                            .get()
                                                                            .as_deref()
                                                                            == Some(id_confirm.as_str())
                                                                    }
                                                                    fallback=move || {
                                                                        let idc = id_cancel.clone();
                                                                        view! {
                                                                            <Button
                                                                                variant=ButtonVariant::Ghost
                                                                                size=ButtonSize::IconSm
                                                                                on:click=move |_| {
                                                                                    deleting_id
                                                                                        .set(
                                                                                            Some(idc.clone()),
                                                                                        )
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
                                                                                let id = id_delete
                                                                                    .clone();
                                                                                move |_| {
                                                                                    delete_action
                                                                                        .dispatch(
                                                                                            DeleteSpot {
                                                                                                spot_id: id
                                                                                                    .clone(),
                                                                                            },
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
                                                        </TableCell>
                                                    </TableRow>
                                                }
                                            })
                                            .collect_view()}
                                    </TableBody>
                                </Table>
                            }
                                .into_any()
                        }}
                    </Suspense>

                    // ── 分页（Suspense 外，信号驱动，响应式更新）────────────
                    {move || {
                        let total = total_items.get();
                        if total == 0 {
                            return view! { <div /> }.into_any();
                        }
                        view! {
                            <div class="flex items-center justify-between px-6 py-4 border-t \
                                        text-sm">
                                <span class="text-muted-foreground">
                                    {move || {
                                        let p = current_page.get();
                                        let t = total_items.get();
                                        format!(
                                            "Showing {}-{} of {} spots",
                                            ((p - 1) * PAGE_SIZE + 1).min(t),
                                            (p * PAGE_SIZE).min(t),
                                            t,
                                        )
                                    }}
                                </span>
                                <div class="flex items-center gap-2">
                                    <button
                                        type="button"
                                        class="px-3 py-1.5 rounded border text-sm \
                                               hover:bg-muted transition-colors \
                                               disabled:opacity-40 disabled:cursor-not-allowed"
                                        disabled=move || current_page.get() <= 1
                                        on:click=move |_| {
                                            current_page
                                                .update(|p| {
                                                    if *p > 1 {
                                                        *p -= 1;
                                                    }
                                                })
                                        }
                                    >
                                        "← Prev"
                                    </button>
                                    <span class="text-muted-foreground tabular-nums px-1">
                                        {move || {
                                            format!(
                                                "{} / {}",
                                                current_page.get(),
                                                total_pages.get(),
                                            )
                                        }}
                                    </span>
                                    <button
                                        type="button"
                                        class="px-3 py-1.5 rounded border text-sm \
                                               hover:bg-muted transition-colors \
                                               disabled:opacity-40 disabled:cursor-not-allowed"
                                        disabled=move || {
                                            current_page.get() >= total_pages.get()
                                        }
                                        on:click=move |_| current_page.update(|p| *p += 1)
                                    >
                                        "Next →"
                                    </button>
                                </div>
                            </div>
                        }
                            .into_any()
                    }}

                </CardContent>
            </Card>

            // ── 删除失败提示 ───────────────────────────────────────────────
            {move || {
                delete_action
                    .value()
                    .get()
                    .and_then(|r| r.err())
                    .map(|e| {
                        view! { <p class="text-sm text-destructive">{e.to_string()}</p> }
                    })
            }}

        </div>
    }
}

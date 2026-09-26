use crate::components::ui::button::{Button, ButtonSize, ButtonVariant};
use crate::components::ui::dialog::{
    Dialog, DialogBody, DialogClose, DialogContent, DialogDescription, DialogFooter, DialogHeader,
    DialogTitle, DialogTrigger,
};
use crate::pages::spots::spot_seasonal_picks::SpotSeasonalPicks;
use crate::server::spots::{
    create_seasonal_picking, delete_seasonal_picking, get_seasonal_picking_types,
    get_seasonal_pickings,
};
use explonz_shared::common::dto::{SeasonalPickingTypeDto, SeasonalPickingsDto};
use icons::{Check, Plus, TimerReset, Trash2};
use leptos::task::spawn_local;
use leptos::{logging, prelude::*};

use crate::pages::spots::MONTHS;

const SELECT_CLS: &str = "text-foreground border-input h-9 rounded-md border bg-transparent \
    dark:bg-input/30 px-2 py-1 text-sm shadow-xs outline-none \
    focus-visible:border-ring focus-visible:ring-ring/50 focus-visible:ring-2 \
    disabled:opacity-50 disabled:cursor-not-allowed";

#[derive(Clone)]
struct DraftPicking {
    local_id: u32,
    type_id: RwSignal<String>,
    start_month: RwSignal<u8>,
    start_day: RwSignal<u8>,
    end_month: RwSignal<u8>,
    end_day: RwSignal<u8>,
}

#[component]
pub fn AddSeasonalPicks(
    spot_id: String,
    seasonal_types: Vec<SeasonalPickingTypeDto>,
) -> impl IntoView {
    let spot_id = StoredValue::new(spot_id);

    let drafts: RwSignal<Vec<DraftPicking>> = RwSignal::new(vec![]);
    let next_id: RwSignal<u32> = RwSignal::new(0);
    // 0 = 未打开过弹框（不发请求），>0 = 每次打开或保存后递增触发刷新
    let fetch_version: RwSignal<u32> = RwSignal::new(0);

    let list_resource = Resource::new(
        move || fetch_version.get(),
        move |version| async move {
            if version == 0 {
                return Ok(vec![]);
            }
            get_seasonal_pickings(spot_id.get_value()).await
        },
    );

    let on_open = Callback::new(move |_| {
        // 每次打开弹框都重新获取 pickings 列表
        fetch_version.update(|v| *v += 1);
    });

    let add_draft = move |_| {
        if !drafts.get_untracked().is_empty() {
            return;
        }
        let id = next_id.get_untracked();
        next_id.update(|n| *n += 1);
        drafts.update(|v| {
            v.push(DraftPicking {
                local_id: id,
                type_id: RwSignal::new(String::new()),
                start_month: RwSignal::new(1),
                start_day: RwSignal::new(1),
                end_month: RwSignal::new(12),
                end_day: RwSignal::new(31),
            });
        });
    };

    view! {
        <Dialog>
            <DialogTrigger
                class="border-0 shadow-none bg-transparent"
                on_open=on_open
            >
                <Button variant=ButtonVariant::Ghost size=ButtonSize::IconSm>
                    <TimerReset class="size-3.5" />
                </Button>
            </DialogTrigger>
            <DialogContent class="max-w-3xl">
                <DialogBody>
                    <DialogHeader>
                        <DialogTitle>"Seasonal Picks"</DialogTitle>
                        <DialogDescription>
                            "Configure what's in season and when at this spot."
                        </DialogDescription>
                    </DialogHeader>

                    <Suspense fallback=move || {
                        view! {
                            <p class="text-sm text-muted-foreground text-center py-2">
                                "Loading..."
                            </p>
                        }
                    }>
                        {move || {
                            match list_resource.get() {
                                None => view! { <></> }.into_any(),
                                Some(Ok(pickings)) => view! {
                                    <SpotSeasonalPicks
                                        seasonal_pickings=pickings
                                        on_delete=Callback::new(move |picking_id: String| {
                                            leptos::task::spawn_local(async move {
                                                match delete_seasonal_picking(picking_id).await {
                                                    Ok(_) => fetch_version.update(|v| *v += 1),
                                                    Err(e) => leptos::logging::log!("Delete failed: {e}"),
                                                }
                                            });
                                        })
                                    />
                                }
                                .into_any(),
                                Some(Err(e)) => view! {
                                    <p class="text-sm text-destructive text-center py-2">
                                        {e.to_string()}
                                    </p>
                                }
                                .into_any(),
                            }
                        }}
                    </Suspense>

                    // Draft rows
                    <div class="flex flex-col gap-2">
                        <For
                            each=move || drafts.get()
                            key=|d| d.local_id
                            children=move |draft| {
                                let lid = draft.local_id;
                                let sid = spot_id.get_value();
                                view! {
                                    <DraftPickingRow
                                        spot_id=sid
                                        draft=draft
                                        seasonal_types=seasonal_types.clone()
                                        on_remove=Callback::new(move |saved: bool| {
                                            drafts.update(|v| v.retain(|d| d.local_id != lid));
                                            if saved {
                                                fetch_version.update(|v| *v += 1);
                                            }
                                        })
                                    />
                                }
                            }
                        />
                    </div>

                    <Button
                        variant=ButtonVariant::Outline
                        class="w-full"
                        attr:disabled=move || !drafts.get().is_empty()
                        on:click=add_draft
                    >
                        <Plus class="size-4" />
                        "Add"
                    </Button>

                    <DialogFooter>
                        <DialogClose>"Close"</DialogClose>
                    </DialogFooter>
                </DialogBody>
            </DialogContent>
        </Dialog>
    }
}

#[component]
fn DraftPickingRow(
    spot_id: String,
    draft: DraftPicking,
    seasonal_types: Vec<SeasonalPickingTypeDto>,
    on_remove: Callback<bool>,
) -> impl IntoView {
    let DraftPicking {
        local_id: _,
        type_id,
        start_month,
        start_day,
        end_month,
        end_day,
    } = draft;

    let save_error: RwSignal<Option<String>> = RwSignal::new(None);

    let save_action: Action<SeasonalPickingsDto, Result<(), ServerFnError>> =
        Action::new(|input: &SeasonalPickingsDto| {
            let spot_id = input.spot_id.clone();
            let type_id = input.type_id.clone();
            let sm = input.start_month;
            let sd = input.start_day;
            let em = input.end_month;
            let ed = input.end_day;
            async move { create_seasonal_picking(spot_id, type_id, sm, sd, em, ed).await }
        });

    Effect::new(move |_| match save_action.value().get() {
        Some(Ok(_)) => on_remove.run(true),
        Some(Err(e)) => save_error.set(Some(e.to_string())),
        None => {}
    });

    let on_save = move |_| {
        let type_id_val = type_id.get_untracked();
        if type_id_val.is_empty() {
            return;
        }
        save_error.set(None);
        save_action.dispatch(SeasonalPickingsDto {
            id: String::new(),
            spot_id: spot_id.clone(),
            type_id: type_id_val,
            type_name: String::new(),
            start_month: start_month.get_untracked() as i16,
            start_day: start_day.get_untracked() as i16,
            end_month: end_month.get_untracked() as i16,
            end_day: end_day.get_untracked() as i16,
        });
    };

    view! {
        <div class="flex flex-col gap-2 rounded-lg border p-3">
            <div class="flex flex-row gap-4 justify-between">

                // Col 1: type selector
                    <div class="flex items-center gap-2">
                        <select
                            class=format!("{SELECT_CLS} flex-1 min-w-0")
                            disabled=move || save_action.pending().get()
                            prop:value=move || type_id.get()
                            on:change=move |e| type_id.set(event_target_value(&e))
                        >
                            <option value="">
                                    "— Select type —"
                            </option>
                            {move || {
                                seasonal_types
                                    .iter()
                                    .map(|t| {
                                        let id = t.id.to_string();
                                        view! { <option value=id>{t.name.clone()}</option> }.into_any()
                                    })
                                    .collect_view()
                            }}
                        </select>
                    </div>

                // Col 2: season date range
                <div class="flex items-center gap-2">
                    <span class="text-muted-foreground text-sm w-10 shrink-0">"From"</span>
                    <select
                        class=SELECT_CLS
                        disabled=move || save_action.pending().get()
                        prop:value=move || start_month.get().to_string()
                        on:change=move |e| {
                            if let Ok(v) = event_target_value(&e).parse::<u8>() {
                                start_month.set(v);
                            }
                        }
                    >
                        {MONTHS
                            .iter()
                            .map(|(name, num)| {
                                let n = *num;
                                view! { <option value=n.to_string()>{*name}</option> }
                            })
                            .collect_view()}
                    </select>
                    <input
                        type="number"
                        class=format!("{SELECT_CLS} w-16 text-center")
                        min="1"
                        max="31"
                        disabled=move || save_action.pending().get()
                        prop:value=move || start_day.get().to_string()
                        on:change=move |e| {
                            if let Ok(v) = event_target_value(&e).parse::<u8>() {
                                start_day.set(v.clamp(1, 31));
                            }
                        }
                    />
                    <span class="text-muted-foreground text-sm shrink-0">"→"</span>
                    <select
                        class=SELECT_CLS
                        disabled=move || save_action.pending().get()
                        prop:value=move || end_month.get().to_string()
                        on:change=move |e| {
                            if let Ok(v) = event_target_value(&e).parse::<u8>() {
                                end_month.set(v);
                            }
                        }
                    >
                        {MONTHS
                            .iter()
                            .map(|(name, num)| {
                                let n = *num;
                                view! { <option value=n.to_string()>{*name}</option> }
                            })
                            .collect_view()}
                    </select>
                    <input
                        type="number"
                        class=format!("{SELECT_CLS} w-16 text-center")
                        min="1"
                        max="31"
                        disabled=move || save_action.pending().get()
                        prop:value=move || end_day.get().to_string()
                        on:change=move |e| {
                            if let Ok(v) = event_target_value(&e).parse::<u8>() {
                                end_day.set(v.clamp(1, 31));
                            }
                        }
                    />
                </div>

                // Col 3: actions
                <div class="flex flex-row gap-2 shrink-0">
                    // Save
                    <Button
                        variant=ButtonVariant::Ghost
                        size=ButtonSize::IconSm
                        attr:disabled=move || {
                            save_action.pending().get() || type_id.get().is_empty()
                        }
                        on:click=on_save
                    >
                        <Check class="size-3.5 text-green-500" />
                    </Button>

                    // Discard
                    <Button
                        variant=ButtonVariant::Ghost
                        size=ButtonSize::IconSm
                        attr:disabled=move || save_action.pending().get()
                        on:click=move |_| on_remove.run(false)
                    >
                        <Trash2 class="size-3.5" />
                    </Button>
                </div>

            </div>

            // Error message
            {move || {
                save_error.get().map(|e| {
                    view! {
                        <p class="text-xs text-destructive">{e}</p>
                    }
                })
            }}
        </div>
    }
}

use crate::components::ui::button::{Button, ButtonSize, ButtonVariant};
use crate::components::ui::dialog::{
    Dialog, DialogBody, DialogClose, DialogContent, DialogDescription, DialogFooter, DialogHeader,
    DialogTitle, DialogTrigger,
};
use crate::server::spots::{create_seasonal_picking, get_seasonal_picking_types};
use explonz_shared::common::dto::SeasonalPickingTypeDto;
use icons::{Check, Plus, TimerReset, Trash2};
use leptos::task::spawn_local;
use leptos::{logging, prelude::*};

const MONTHS: [(&str, u8); 12] = [
    ("Jan", 1),
    ("Feb", 2),
    ("Mar", 3),
    ("Apr", 4),
    ("May", 5),
    ("Jun", 6),
    ("Jul", 7),
    ("Aug", 8),
    ("Sep", 9),
    ("Oct", 10),
    ("Nov", 11),
    ("Dec", 12),
];

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

struct SavePickingInput {
    spot_id: String,
    type_id: String,
    start_month: i16,
    start_day: i16,
    end_month: i16,
    end_day: i16,
}

#[component]
pub fn SeasonalPicks(
    spot_id: String,
    seasonal_types: ReadSignal<Option<Vec<SeasonalPickingTypeDto>>>,
    on_load: Callback<Vec<SeasonalPickingTypeDto>>,
) -> impl IntoView {
    let spot_id = StoredValue::new(spot_id);

    let drafts: RwSignal<Vec<DraftPicking>> = RwSignal::new(vec![]);
    let next_id: RwSignal<u32> = RwSignal::new(0);

    let on_open = Callback::new(move |_| {
        if seasonal_types.get_untracked().is_some() {
            return;
        }
        spawn_local(async move {
            match get_seasonal_picking_types().await {
                Ok(data) => {
                    logging::log!("seasonal_picking_types_data: {:?}", data);
                    on_load.run(data);
                }
                Err(err) => {
                    logging::log!("Failed to load seasonal picking types: {err}");
                }
            }
        });
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

                    // TODO: load & display existing saved pickings from server

                    // Draft rows / empty state
                    {move || {
                        if drafts.get().is_empty() {
                            view! {
                                <p class="text-sm text-muted-foreground text-center py-4 border rounded-lg">
                                    "No seasonal picks yet. Click \"Add\" to get started."
                                </p>
                            }
                            .into_any()
                        } else {
                            view! {
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
                                                    seasonal_types=seasonal_types
                                                    on_remove=Callback::new(move |_| {
                                                        drafts
                                                            .update(|v| {
                                                                v.retain(|d| d.local_id != lid)
                                                            });
                                                    })
                                                />
                                            }
                                        }
                                    />
                                </div>
                            }
                            .into_any()
                        }
                    }}

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
    seasonal_types: ReadSignal<Option<Vec<SeasonalPickingTypeDto>>>,
    on_remove: Callback<()>,
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

    let save_action: Action<SavePickingInput, Result<(), ServerFnError>> =
        Action::new(|input: &SavePickingInput| {
            let spot_id = input.spot_id.clone();
            let type_id = input.type_id.clone();
            let sm = input.start_month;
            let sd = input.start_day;
            let em = input.end_month;
            let ed = input.end_day;
            async move { create_seasonal_picking(spot_id, type_id, sm, sd, em, ed).await }
        });

    Effect::new(move |_| match save_action.value().get() {
        Some(Ok(_)) => on_remove.run(()),
        Some(Err(e)) => save_error.set(Some(e.to_string())),
        None => {}
    });

    let on_save = move |_| {
        let type_id_val = type_id.get_untracked();
        if type_id_val.is_empty() {
            return;
        }
        save_error.set(None);
        save_action.dispatch(SavePickingInput {
            spot_id: spot_id.clone(),
            type_id: type_id_val,
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
                        disabled=move || seasonal_types.get().is_none() || save_action.pending().get()
                        prop:value=move || type_id.get()
                        on:change=move |e| type_id.set(event_target_value(&e))
                    >
                        <option value="">
                            {move || {
                                if seasonal_types.get().is_none() {
                                    "Loading..."
                                } else {
                                    "— Select type —"
                                }
                            }}
                        </option>
                        {move || {
                            seasonal_types
                                .get()
                                .unwrap_or_default()
                                .into_iter()
                                .map(|t| {
                                    let id = t.id.to_string();
                                    view! { <option value=id>{t.name}</option> }
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
                        on:click=move |_| on_remove.run(())
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

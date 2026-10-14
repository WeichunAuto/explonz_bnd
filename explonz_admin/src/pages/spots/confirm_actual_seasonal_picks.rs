use crate::components::ui::button::{Button, ButtonSize, ButtonVariant};
use crate::components::ui::input::Input;
use crate::components::ui::date_picker::{
    DatePicker, DatePickerCell, DatePickerHeader, DatePickerMonth, DatePickerRow, DatePickerTable,
    DatePickerTitle, DatePickerWeekDay,
};
use crate::components::ui::dialog::{
    use_dialog_trigger_id, Dialog, DialogBody, DialogClose, DialogContent, DialogDescription,
    DialogFooter, DialogHeader, DialogTitle, DialogTrigger,
};
use crate::server::spots::upsert_actual_seasonal_pick;
use explonz_shared::common::dto::ActualSeasonalPickDto;
use leptos::prelude::*;
use time::{Date, Duration, Month, OffsetDateTime};

#[component]
pub fn ConfirmActualSeasonalPicks(picking_id: String) -> impl IntoView {
    let toast_ref: NodeRef<leptos::html::Button> = NodeRef::new();

    view! {
        <Dialog>
            <DialogTrigger class="border-0 shadow-none bg-transparent">
                "Confirm"
            </DialogTrigger>
            <DialogContent class="max-w-md">
                <ConfirmContent picking_id=picking_id />
            </DialogContent>
        </Dialog>
    }
}

// Inner component rendered inside <Dialog>, so it has access to DialogContext
#[component]
fn ConfirmContent(picking_id: String) -> impl IntoView {
    // use_dialog_trigger_id() is available here because we're inside <Dialog>
    let dialog_id = use_dialog_trigger_id().unwrap_or_default();

    // 初始展示显示 今日 以及 10天后的日期
    let today = OffsetDateTime::now_utc().date();
    let end_date = today + Duration::days(10);
    let start_date_signal = RwSignal::new(today);
    let end_date_signal = RwSignal::new(end_date);

    // Which month/year is currently displayed in the calendar
    let view_year: RwSignal<i32> = RwSignal::new(start_date_signal.get_untracked().year());
    let view_month: RwSignal<Month> = RwSignal::new(start_date_signal.get_untracked().month());

    // true = next click sets start date; false = next click sets end date
    let selecting_start: RwSignal<bool> = RwSignal::new(true);

    let notes_signal: RwSignal<String> = RwSignal::new(String::new());

    let error_msg: RwSignal<Option<String>> = RwSignal::new(None);
    let success_msg: RwSignal<Option<String>> = RwSignal::new(None);

    let picking_id_for_action = picking_id.clone();
    let save_action: Action<(), Result<ActualSeasonalPickDto, ServerFnError>> =
        Action::new(move |_: &()| {
            let pid = picking_id_for_action.clone();
            let start = start_date_signal.get_untracked().to_string();
            let end = end_date_signal.get_untracked().to_string();
            let notes = notes_signal.get_untracked();
            async move { upsert_actual_seasonal_pick(pid, start, end, notes).await }
        });

    Effect::new(move |_| match save_action.value().get() {
        Some(Ok(data)) => {
            error_msg.set(None);
            success_msg.set(Some(format!("设置成功！")));
            leptos::logging::log!("data = {:?}", data);
        }
        Some(Err(e)) => error_msg.set(Some(e.to_string())),
        None => {}
    });

    let on_confirm = move |_| {
        error_msg.set(None);
        save_action.dispatch(());
    };

    let prev_month = move |_| {
        let m = view_month.get_untracked();
        let y = view_year.get_untracked();
        if m == Month::January {
            view_month.set(Month::December);
            view_year.set(y - 1);
        } else {
            view_month.set(m.previous());
        }
    };

    let next_month = move |_| {
        let m = view_month.get_untracked();
        let y = view_year.get_untracked();
        if m == Month::December {
            view_month.set(Month::January);
            view_year.set(y + 1);
        } else {
            view_month.set(m.next());
        }
    };

    view! {
        <DialogBody>
            <DialogHeader>
                <DialogTitle>"Confirm Seasonal Picks"</DialogTitle>
                <DialogDescription>
                    "Configure what's in season and when at this spot..."
                </DialogDescription>
            </DialogHeader>
            <div class="w-full flex flex-col justify-center items-center gap-4 py-4">
                <DatePicker>
                    <DatePickerHeader>
                        // clx!-generated components don't forward on:click, use Button instead
                        <Button
                            variant=ButtonVariant::Ghost
                            size=ButtonSize::IconSm
                            attr:class="border border-input opacity-50 hover:opacity-100"
                            on:click=prev_month
                        >
                            "‹"
                        </Button>
                        <DatePickerTitle>
                            {move || format!("{} {}", view_month.get(), view_year.get())}
                        </DatePickerTitle>
                        <Button
                            variant=ButtonVariant::Ghost
                            size=ButtonSize::IconSm
                            attr:class="border border-input opacity-50 hover:opacity-100"
                            on:click=next_month
                        >
                            "›"
                        </Button>
                    </DatePickerHeader>
                    <DatePickerMonth>
                        <DatePickerTable>
                            <thead>
                                <DatePickerRow>
                                    <DatePickerWeekDay>"Su"</DatePickerWeekDay>
                                    <DatePickerWeekDay>"Mo"</DatePickerWeekDay>
                                    <DatePickerWeekDay>"Tu"</DatePickerWeekDay>
                                    <DatePickerWeekDay>"We"</DatePickerWeekDay>
                                    <DatePickerWeekDay>"Th"</DatePickerWeekDay>
                                    <DatePickerWeekDay>"Fr"</DatePickerWeekDay>
                                    <DatePickerWeekDay>"Sa"</DatePickerWeekDay>
                                </DatePickerRow>
                            </thead>
                            <tbody>
                                {move || {
                                    // 读取 selection signals：保证选择变化时整个 grid 重新渲染，
                                    // 使每个 cell 重新计算 aria-current 和 class
                                    let _ = start_date_signal.get();
                                    let _ = end_date_signal.get();

                                    let y = view_year.get();
                                    let m = view_month.get();

                                    // 0 = Sun, 1 = Mon, …, 6 = Sat  (leading empty cells)
                                    let leading = Date::from_calendar_date(y, m, 1)
                                        .unwrap()
                                        .weekday()
                                        .number_days_from_sunday() as usize;

                                    let days_count =
                                        time::util::days_in_year_month(y, m) as usize;
                                    let row_count = (leading + days_count + 6) / 7;

                                    (0..row_count)
                                        .map(|row| {
                                            view! {
                                                <DatePickerRow>
                                                    {(0..7usize)
                                                        .map(|col| {
                                                            let pos = row * 7 + col;
                                                            if pos < leading
                                                                || pos >= leading + days_count
                                                            {
                                                                // Empty padding cell
                                                                view! { <td class="size-9" /> }
                                                                    .into_any()
                                                            } else {
                                                                let day =
                                                                    (pos - leading + 1) as u8;
                                                                view! {
                                                                    <DatePickerCell
                                                                        day=day
                                                                        year=y
                                                                        month=m
                                                                        disabled=false
                                                                        start_date=start_date_signal
                                                                        end_date=end_date_signal
                                                                        on_click=move |d| {
                                                                            if let Ok(date) =
                                                                                Date::from_calendar_date(
                                                                                    y, m, d,
                                                                                )
                                                                            {
                                                                                if selecting_start
                                                                                    .get_untracked()
                                                                                {
                                                                                    start_date_signal
                                                                                        .set(date);
                                                                                    end_date_signal
                                                                                        .set(date);
                                                                                    selecting_start
                                                                                        .set(false);
                                                                                } else {
                                                                                    let start =
                                                                                        start_date_signal
                                                                                            .get_untracked();
                                                                                    if date >= start {
                                                                                        end_date_signal
                                                                                            .set(date);
                                                                                    } else {
                                                                                        start_date_signal
                                                                                            .set(date);
                                                                                        end_date_signal
                                                                                            .set(start);
                                                                                    }
                                                                                    selecting_start
                                                                                        .set(true);
                                                                                }
                                                                            }
                                                                        }
                                                                    />
                                                                }
                                                                .into_any()
                                                            }
                                                        })
                                                        .collect_view()}
                                                </DatePickerRow>
                                            }
                                        })
                                        .collect_view()
                                }}
                            </tbody>
                        </DatePickerTable>
                    </DatePickerMonth>
                </DatePicker>

                // Selected range display
                <div class="flex items-center gap-2 text-sm text-muted-foreground">
                    <span class="font-medium text-foreground">"Start:"</span>
                    <span>{move || start_date_signal.get().to_string()}</span>
                    <span>"–"</span>
                    <span class="font-medium text-foreground">"End:"</span>
                    <span>{move || end_date_signal.get().to_string()}</span>
                </div>
            </div>

            <div class="grid gap-1.5">
                <label class="text-sm font-medium">
                    "Notes "
                    <span class="font-normal text-muted-foreground">"(optional)"</span>
                </label>
                <Input
                    placeholder="e.g. Confirmed by farm owner"
                    bind_value=notes_signal
                />
            </div>

            {move || error_msg.get().map(|e| view! {
                <p class="text-sm text-destructive">{e}</p>
            })}
        
        // 提交成功后，显示成功消息。
        {move || match success_msg.get() {
            Some(msg) => {
                leptos::logging::log!("success_msg = {:?}", msg);
        
                view! {
                    <p class="text-sm text-green-600" role="status">
                        {msg}
                    </p>
                }
                .into_any()
            }
            None => ().into_any(),
        }}

            <DialogFooter>
                <Button
                    on:click=on_confirm
                    attr:disabled=move || save_action.pending().get()
                >
                    {move || if save_action.pending().get() { "Saving…" } else { "Confirm" }}
                </Button>
                <DialogClose>"Close"</DialogClose>
            </DialogFooter>


        </DialogBody>
    }
}

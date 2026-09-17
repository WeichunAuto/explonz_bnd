use crate::components::ui::button::{Button, ButtonSize, ButtonVariant};
use crate::components::ui::dialog::{
    Dialog, DialogAction, DialogBody, DialogClose, DialogContent, DialogDescription, DialogFooter,
    DialogHeader, DialogTitle, DialogTrigger,
};
use crate::server::spots::get_seasonal_picking_types;
use explonz_shared::common::dto::SeasonalPickingTypeDto;
use icons::TimerReset;
use leptos::task::spawn_local;
use leptos::{logging, prelude::*};

#[component]
pub fn SeasonalPicks(
    spot_id: String,
    seasonal_types: ReadSignal<Option<Vec<SeasonalPickingTypeDto>>>,
    on_load: Callback<Vec<SeasonalPickingTypeDto>>,
) -> impl IntoView {
    // 当对话框打开时的回调函数
    let on_open = Callback::new(move |_| {
        // 如果数据已加载，则忽略
        if seasonal_types.get_untracked().is_some() {
            return;
        }
        spawn_local(async move {
            match get_seasonal_picking_types().await {
                Ok(data) => {
                    // logging::log!("seasonal_picking_types_data: {:?}", data);
                    on_load.run(data);
                }
                Err(err) => {
                    logging::log!("Failed to load seasonal picking types: {err}");
                }
            }
        });
    });

    view! {
        <Dialog>
            <DialogTrigger
                class="border-0 shadow-none bg-transparent size-3.5"
                on_open=on_open
            >
                <Button
                    variant=ButtonVariant::Ghost
                    size=ButtonSize::IconSm
                >
                    <TimerReset class="size-3.5" />
                </Button>
            </DialogTrigger>
            <DialogContent class="max-w-xl">
                <DialogBody>
                    <DialogHeader>
                        <DialogTitle>"Dialog Title"</DialogTitle>
                        <DialogDescription>"Dialog Description"</DialogDescription>
                    </DialogHeader>
                    <DialogFooter>
                        <DialogClose>"Cancel"</DialogClose>
                        <DialogAction>"Confirm"</DialogAction>
                    </DialogFooter>
                </DialogBody>
            </DialogContent>
        </Dialog>
    }
}

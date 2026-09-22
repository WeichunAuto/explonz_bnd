use crate::components::ui::button::{Button, ButtonSize, ButtonVariant};
use crate::components::ui::table::{Table, TableBody, TableCell, TableHead, TableHeader, TableRow};
use crate::pages::spots::month_short;
use explonz_shared::common::dto::SeasonalPickingsDto;
use icons::Trash2;
use leptos::prelude::*;

#[component]
pub fn SeasonalPicksList(
    seasonal_pickings: Vec<SeasonalPickingsDto>,
    on_delete: Callback<String>,
) -> impl IntoView {
    if seasonal_pickings.is_empty() {
        return view! {
            <p class="text-sm text-muted-foreground text-center py-4 border rounded-lg">
                "No seasonal picks yet. Click \"Add\" to get started."
            </p>
        }
        .into_any();
    }

    // Track which row is pending confirmation (by picking id)
    let pending_id: RwSignal<Option<String>> = RwSignal::new(None);

    view! {
        <Table>
            <TableHeader>
                <TableRow>
                    <TableHead>"No"</TableHead>
                    <TableHead>"Type"</TableHead>
                    <TableHead>"Season"</TableHead>
                    <TableHead>""</TableHead>
                </TableRow>
            </TableHeader>
            <TableBody>
                {seasonal_pickings
                    .into_iter()
                    .enumerate()
                    .map(|(i, sp)| {
                        let season = format!(
                            "{} {} → {} {}",
                            month_short(sp.start_month),
                            sp.start_day,
                            month_short(sp.end_month),
                            sp.end_day,
                        );
                        let picking_id = sp.id.clone();
                        view! {
                            <TableRow>
                                <TableCell class="font-medium">{i + 1}</TableCell>
                                <TableCell class="font-medium">{sp.type_name}</TableCell>
                                <TableCell class="text-muted-foreground">{season}</TableCell>
                                <TableCell class="text-right">
                                    {move || {
                                        let id = picking_id.clone();
                                        if pending_id.get().as_deref() == Some(&id) {
                                            // Confirm row
                                            let id_confirm = id.clone();
                                            let id_cancel = id.clone();
                                            view! {
                                                <div class="flex items-center justify-end gap-1">
                                                    <span class="text-xs text-muted-foreground mr-1">
                                                        "Delete?"
                                                    </span>
                                                    <Button
                                                        variant=ButtonVariant::Destructive
                                                        size=ButtonSize::Sm
                                                        on:click=move |_| {
                                                            pending_id.set(None);
                                                            on_delete.run(id_confirm.clone());
                                                        }
                                                    >
                                                        "Yes"
                                                    </Button>
                                                    <Button
                                                        variant=ButtonVariant::Ghost
                                                        size=ButtonSize::Sm
                                                        on:click=move |_| pending_id.set(None)
                                                    >
                                                        "No"
                                                    </Button>
                                                </div>
                                            }
                                            .into_any()
                                        } else {
                                            let id_trash = id.clone();
                                            view! {
                                                <Button
                                                    variant=ButtonVariant::Ghost
                                                    size=ButtonSize::IconSm
                                                    on:click=move |_| {
                                                        pending_id.set(Some(id_trash.clone()))
                                                    }
                                                >
                                                    <Trash2 class="size-3.5 text-destructive" />
                                                </Button>
                                            }
                                            .into_any()
                                        }
                                    }}
                                </TableCell>
                            </TableRow>
                        }
                    })
                    .collect_view()}
            </TableBody>
        </Table>
    }
    .into_any()
}

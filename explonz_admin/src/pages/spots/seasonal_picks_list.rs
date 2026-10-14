use super::month_short;
use crate::components::ui::accordion::{
    Accordion, AccordionContent, AccordionItem, AccordionTrigger,
};
use crate::components::ui::button::{Button, ButtonSize, ButtonVariant};
use crate::pages::spots::confirm_actual_seasonal_picks::ConfirmActualSeasonalPicks;
use crate::server::spots::get_all_seasonal_pickings_with_spots;
use leptos::prelude::*;

#[component]
pub fn SeasonalPicksList() -> impl IntoView {
    let picks_resource = Resource::new(|| (), |_| get_all_seasonal_pickings_with_spots());

    view! {
        <div class="flex flex-col gap-3">
            <Suspense fallback=|| {
                view! { <div class="text-sm text-muted-foreground p-4">"Loading..."</div> }
            }>
                {move || {
                    let result = picks_resource.get();
                    match result {
                        None => view! { <div /> }.into_any(),
                        Some(Err(e)) => {
                            view! {
                                <p class="text-sm text-destructive p-4">{e.to_string()}</p>
                            }
                                .into_any()
                        }
                        Some(Ok(picks)) => {
                            let type_count = picks.len();
                            view! {
                                <div class="flex flex-col gap-3">
                                    // Header
                                    <div class="flex items-center justify-between">
                                        <h2 class="text-base font-semibold">"Seasonal Picks"</h2>
                                        <span class="text-xs text-muted-foreground">
                                            {format!("{} types", type_count)}
                                        </span>
                                    </div>

                                    // Column header bar
                                    <div class="grid grid-cols-[1fr_auto_auto] gap-4 px-3 py-1.5 text-xs font-medium text-muted-foreground border-b">
                                        <span>"Type"</span>
                                        <span class="w-40 text-center">"Season"</span>
                                        <span class="w-16 text-center">"Spots"</span>
                                    </div>

                                    // Accordion list
                                    <Accordion>
                                        {picks
                                            .into_iter()
                                            .map(|pt| {
                                                let season = format!(
                                                    "{} {} – {} {}",
                                                    month_short(pt.start_month),
                                                    pt.start_day,
                                                    month_short(pt.end_month),
                                                    pt.end_day,
                                                );
                                                let spot_count = pt.spots.len();
                                                view! {
                                                    <AccordionItem>
                                                        <AccordionTrigger class="py-1">
                                                            <div class="grid grid-cols-[1fr_auto_auto] gap-4 w-full items-center">
                                                                <span class="text-sm font-medium">
                                                                    {pt.type_name}
                                                                </span>
                                                                <span class="w-40 text-center text-sm text-muted-foreground">
                                                                    {season}
                                                                </span>
                                                                <span class="w-16 text-center">
                                                                    <span class="inline-flex items-center justify-center rounded-full bg-muted px-2 text-xs font-medium">
                                                                        {spot_count}
                                                                    </span>
                                                                </span>
                                                            </div>
                                                        </AccordionTrigger>

                                                        <AccordionContent>
                                                            <div class="rounded-md border overflow-hidden">
                                                                <table class="w-full text-sm">
                                                                    <thead>
                                                                        <tr class="border-b bg-muted/40">
                                                                            <th class="px-3 py-2 text-left text-xs font-medium text-muted-foreground">
                                                                                "Spot"
                                                                            </th>
                                                                            <th class="px-3 py-2 text-left text-xs font-medium text-muted-foreground">
                                                                                "Location"
                                                                            </th>
                                                                            <th class="px-3 py-2 text-left text-xs font-medium text-muted-foreground">
                                                                                "Season Picks"
                                                                            </th>
                                                                            <th class="px-3 py-2 text-right text-xs font-medium text-muted-foreground">
                                                                                "Rating"
                                                                            </th>
                                                                            <th class="px-3 py-2 text-right text-xs font-medium text-muted-foreground">
                                                                                ""
                                                                            </th>
                                                                        </tr>
                                                                    </thead>
                                                                    <tbody>
                                                                        {pt
                                                                            .spots
                                                                            .into_iter()
                                                                            .map(|spot| {
                                                                                view! {
                                                                                    <tr class="border-b last:border-0 hover:bg-muted/30 transition-colors">
                                                                                        <td class="px-3 py-2 font-medium">
                                                                                            {spot.spot_name}
                                                                                        </td>
                                                                                        <td class="px-3 py-2 text-muted-foreground">
                                                                                            {spot.location}
                                                                                        </td>
                                                                                        <td class="px-3 py-2 text-muted-foreground">
                                                                                            {spot.created_at}
                                                                                        </td>
                                                                                        <td class="px-3 py-2 text-right">
                                                                                            <span class="inline-flex items-center gap-1 text-amber-500 font-medium">
                                                                                                "★ "
                                                                                                {spot.rating}
                                                                                            </span>
                                                                                        </td>
                                                                                        <td class="px-3 py-2 text-right">

                                                                                            <ConfirmActualSeasonalPicks picking_id=spot.picking_id/>
                                                                                        </td>
                                                                                    </tr>
                                                                                }
                                                                            })
                                                                            .collect_view()}
                                                                    </tbody>
                                                                </table>
                                                            </div>
                                                        </AccordionContent>
                                                    </AccordionItem>
                                                }
                                            })
                                            .collect_view()}
                                    </Accordion>
                                </div>
                            }
                                .into_any()
                        }
                    }
                }}
            </Suspense>
        </div>
    }
}

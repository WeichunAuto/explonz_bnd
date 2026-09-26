use crate::components::ui::accordion::{
    Accordion, AccordionContent, AccordionItem, AccordionTrigger,
};
use crate::components::ui::button::{Button, ButtonSize, ButtonVariant};
use leptos::prelude::*;

use super::{month_short, MONTHS};

// ── Mock data types ──────────────────────────────────────────────────────────

struct MockSpot {
    name: &'static str,
    location: &'static str,
    rating: &'static str,
}

struct MockPickingType {
    id: &'static str,
    name: &'static str,
    start_month: u8,
    start_day: u8,
    end_month: u8,
    end_day: u8,
    spots: &'static [MockSpot],
}

static MOCK_DATA: &[MockPickingType] = &[
    MockPickingType {
        id: "1",
        name: "Strawberry",
        start_month: 4,
        start_day: 1,
        end_month: 6,
        end_day: 30,
        spots: &[
            MockSpot {
                name: "Sunny Berry Farm",
                location: "Tochigi, Japan",
                rating: "4.8",
            },
            MockSpot {
                name: "Green Meadow Farm",
                location: "Chiba, Japan",
                rating: "4.5",
            },
            MockSpot {
                name: "Hilltop Orchard",
                location: "Kanagawa, Japan",
                rating: "4.2",
            },
        ],
    },
    MockPickingType {
        id: "2",
        name: "Blueberry",
        start_month: 6,
        start_day: 15,
        end_month: 8,
        end_day: 31,
        spots: &[
            MockSpot {
                name: "Blue Sky Farm",
                location: "Nagano, Japan",
                rating: "4.6",
            },
            MockSpot {
                name: "Forest Berry Garden",
                location: "Yamanashi, Japan",
                rating: "4.3",
            },
        ],
    },
    MockPickingType {
        id: "3",
        name: "Grape",
        start_month: 8,
        start_day: 1,
        end_month: 10,
        end_day: 31,
        spots: &[
            MockSpot {
                name: "Chateau Vineyard",
                location: "Yamanashi, Japan",
                rating: "4.9",
            },
            MockSpot {
                name: "Valley Grape Farm",
                location: "Nagano, Japan",
                rating: "4.7",
            },
            MockSpot {
                name: "Sunrise Winery",
                location: "Hokkaido, Japan",
                rating: "4.4",
            },
            MockSpot {
                name: "Golden Harvest",
                location: "Okayama, Japan",
                rating: "4.1",
            },
        ],
    },
    MockPickingType {
        id: "4",
        name: "Apple",
        start_month: 9,
        start_day: 1,
        end_month: 11,
        end_day: 30,
        spots: &[
            MockSpot {
                name: "Red Apple Orchard",
                location: "Aomori, Japan",
                rating: "4.8",
            },
            MockSpot {
                name: "Autumn Hill Farm",
                location: "Iwate, Japan",
                rating: "4.5",
            },
        ],
    },
];

// ── Component ────────────────────────────────────────────────────────────────

#[component]
pub fn SeasonalPicksList() -> impl IntoView {
    view! {
        <div class="flex flex-col gap-3">
            // Header
            <div class="flex items-center justify-between">
                <h2 class="text-base font-semibold">"Seasonal Picks"</h2>
                <span class="text-xs text-muted-foreground">
                    {format!("{} types", MOCK_DATA.len())}
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
                {MOCK_DATA
                    .iter()
                    .map(|pt| {
                        let season = format!(
                            "{} {} – {} {}",
                            month_short(pt.start_month as i16),
                            pt.start_day,
                            month_short(pt.end_month as i16),
                            pt.end_day,
                        );
                        let spot_count = pt.spots.len();
                        view! {
                            <AccordionItem>
                                <AccordionTrigger class="py-1">
                                    // Trigger inner layout mirrors the column header
                                    <div class="grid grid-cols-[1fr_auto_auto] gap-4 w-full items-center">
                                        <span class="text-sm font-medium">{pt.name}</span>
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
                                                    .iter()
                                                    .map(|spot| {
                                                        view! {
                                                            <tr class="border-b last:border-0 hover:bg-muted/30 transition-colors">
                                                                <td class="px-3 py-2 font-medium">
                                                                    {spot.name}
                                                                </td>
                                                                <td class="px-3 py-2 text-muted-foreground">
                                                                    {spot.location}
                                                                </td>
                                                                <td class="px-3 py-2 text-right">
                                                                    <span class="inline-flex items-center gap-1 text-amber-500 font-medium">
                                                                        "★ "
                                                                        {spot.rating}
                                                                    </span>
                                                                </td>
                                                                <td class="px-3 py-2 text-right">
                                                                    <Button
                                                                        variant=ButtonVariant::Ghost
                                                                        size=ButtonSize::Sm
                                                                        attr:class="text-xs h-7"
                                                                    >
                                                                        "View"
                                                                    </Button>
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
}

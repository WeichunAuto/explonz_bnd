use explonz_shared::common::dto::SpotDto;
use icons::ScanEye;
use leptos::prelude::*;

use crate::components::ui::button::{Button, ButtonSize, ButtonVariant};
use crate::components::ui::sheet::{
    Sheet, SheetBody, SheetClose, SheetContent, SheetDescription, SheetDirection, SheetFooter,
    SheetHeader, SheetTitle, SheetTrigger,
};

#[component]
pub fn SpotDetail(spot_detail: Option<SpotDto>) -> impl IntoView {
    // 提取字段，None 时使用空字符串作为降级
    let s = spot_detail.unwrap_or_default();

    let name = s.name.clone();
    let description = s.description.clone();
    let location2 = s.location.clone();
    let location = s.location.clone();
    let rating = s.rating.to_string();
    let latitude = format!("{:.6}", s.latitude);
    let longitude = format!("{:.6}", s.longitude);
    let phone = s.phone.clone();
    let website = s.website.clone();
    let photo_urls = s.photo_urls.clone();
    let labels = s.labels.clone();
    let opening_hours = s.opening_hours.clone();
    let created_at = s.created_at.format("%Y-%m-%d %H:%M").to_string();
    let updated_at = s.updated_at.format("%Y-%m-%d %H:%M").to_string();
    let spot_id = s.id.to_string();

    view! {
        <Sheet>

            // ==============================
            // Trigger
            // ==============================
            <SheetTrigger class="border-0 shadow-none bg-transparent size-3.5">
                <Button
                    variant=ButtonVariant::Ghost
                    size=ButtonSize::IconSm
                >
                    <ScanEye class="size-3.5" />
                </Button>
            </SheetTrigger>

            // ==============================
            // Sheet Content
            // ==============================
            <SheetContent direction=SheetDirection::Right>

                // ==============================
                // Header
                // ==============================
                <SheetHeader>
                    <SheetTitle>
                        {name}
                    </SheetTitle>

                    <SheetDescription>
                        <span class="text-black">{location2}</span>
                    </SheetDescription>
                </SheetHeader>

                // ==============================
                // Body
                // ==============================
                <SheetBody>

                    <div class="space-y-6">

                        // Photos
                        {if !photo_urls.is_empty() {
                            view! {
                                <div class="grid grid-cols-3 gap-2">
                                    {photo_urls
                                        .into_iter()
                                        .map(|url| {
                                            view! {
                                                <img
                                                    src=url
                                                    class="w-full aspect-square object-cover rounded-md"
                                                    alt="spot photo"
                                                />
                                            }
                                        })
                                        .collect_view()}
                                </div>
                            }.into_any()
                        } else {
                            view! {
                                <div class="w-full h-32 bg-muted rounded-md flex items-center justify-center text-sm text-muted-foreground">
                                    "No photos"
                                </div>
                            }.into_any()
                        }}
                        // Description
                        {if !description.is_empty() {
                            view! {
                                <div class="space-y-1">
                                    <div class="text-xs font-medium text-muted-foreground uppercase tracking-wide">
                                        "Description"
                                    </div>
                                    <div class="text-sm leading-relaxed">
                                        {description}
                                    </div>
                                </div>
                            }.into_any()
                        } else {
                            view! { <div /> }.into_any()
                        }}

                        // Rating & Location
                        <div class="grid grid-cols-2 gap-4">
                            <div class="space-y-1">
                                <div class="text-xs font-medium text-muted-foreground uppercase tracking-wide">
                                    "Rating"
                                </div>
                                <div class="text-sm">
                                    {rating}
                                </div>
                            </div>
                            <div class="space-y-1">
                                <div class="text-xs font-medium text-muted-foreground uppercase tracking-wide">
                                    "Location"
                                </div>
                                <div class="text-sm break-words">
                                    {location}
                                </div>
                            </div>
                        </div>

                        // Coordinates
                        <div class="grid grid-cols-2 gap-4">
                            <div class="space-y-1">
                                <div class="text-xs font-medium text-muted-foreground uppercase tracking-wide">
                                    "Latitude"
                                </div>
                                <div class="text-sm font-mono">
                                    {latitude}
                                </div>
                            </div>
                            <div class="space-y-1">
                                <div class="text-xs font-medium text-muted-foreground uppercase tracking-wide">
                                    "Longitude"
                                </div>
                                <div class="text-sm font-mono">
                                    {longitude}
                                </div>
                            </div>
                        </div>

                        // Phone
                        {if let Some(ph) = phone {
                            view! {
                                <div class="space-y-1">
                                    <div class="text-xs font-medium text-muted-foreground uppercase tracking-wide">
                                        "Phone"
                                    </div>
                                    <div class="text-sm">
                                        {ph}
                                    </div>
                                </div>
                            }.into_any()
                        } else {
                            view! { <div /> }.into_any()
                        }}

                        // Website
                        {if let Some(ws) = website {
                            view! {
                                <div class="space-y-1">
                                    <div class="text-xs font-medium text-muted-foreground uppercase tracking-wide">
                                        "Website"
                                    </div>
                                    <div class="text-sm break-all text-muted-foreground">
                                        {ws}
                                    </div>
                                </div>
                            }.into_any()
                        } else {
                            view! { <div /> }.into_any()
                        }}

                        // Labels
                        {if !labels.is_empty() {
                            view! {
                                <div class="space-y-2">
                                    <div class="text-xs font-medium text-muted-foreground uppercase tracking-wide">
                                        "Labels"
                                    </div>
                                    <div class="flex flex-wrap gap-1.5">
                                        {labels
                                            .into_iter()
                                            .map(|l| {
                                                view! {
                                                    <span class="rounded-md bg-secondary px-2 py-1 text-xs">
                                                        {l.name}
                                                    </span>
                                                }
                                            })
                                            .collect_view()}
                                    </div>
                                </div>
                            }.into_any()
                        } else {
                            view! { <div /> }.into_any()
                        }}

                        // Opening Hours
                        {if !opening_hours.is_empty() {
                            const DAY_NAMES: [&str; 7] =
                                ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
                            view! {
                                <div class="space-y-2">
                                    <div class="text-xs font-medium text-muted-foreground uppercase tracking-wide">
                                        "Opening Hours"
                                    </div>
                                    <div class="space-y-1">
                                        {opening_hours
                                            .into_iter()
                                            .map(|h| {
                                                let day = DAY_NAMES
                                                    .get(h.day_of_week as usize)
                                                    .copied()
                                                    .unwrap_or("?");
                                                let hours_text = if h.is_closed {
                                                    "Closed".to_string()
                                                } else if h.is_open_24h {
                                                    "Open 24h".to_string()
                                                } else {
                                                    format!(
                                                        "{} – {}",
                                                        h.open_time.as_deref().unwrap_or("?"),
                                                        h.close_time.as_deref().unwrap_or("?"),
                                                    )
                                                };
                                                view! {
                                                    <div class="flex justify-between text-sm">
                                                        <span class="text-muted-foreground w-10 shrink-0">
                                                            {day}
                                                        </span>
                                                        <span class="text-right">
                                                            {hours_text}
                                                        </span>
                                                    </div>
                                                }
                                            })
                                            .collect_view()}
                                    </div>
                                </div>
                            }.into_any()
                        } else {
                            view! { <div /> }.into_any()
                        }}

                        // Timestamps
                        <div class="grid grid-cols-2 gap-4 pb-2 border-b">
                            <div class="space-y-1">
                                <div class="text-xs font-medium text-muted-foreground uppercase tracking-wide">
                                    "Created"
                                </div>
                                <div class="text-xs text-muted-foreground">
                                    {created_at}
                                </div>
                            </div>
                            <div class="space-y-1">
                                <div class="text-xs font-medium text-muted-foreground uppercase tracking-wide">
                                    "Updated"
                                </div>
                                <div class="text-xs text-muted-foreground">
                                    {updated_at}
                                </div>
                            </div>
                        </div>

                    </div>

                </SheetBody>

                // ==============================
                // Footer
                // ==============================
                <SheetFooter class="items-end">

                    <SheetClose variant=ButtonVariant::Outline >
                        "Close"
                    </SheetClose>

                </SheetFooter>

            </SheetContent>

        </Sheet>
    }
}

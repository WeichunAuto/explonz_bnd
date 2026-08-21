use icons::{ChevronsUpDown, LayoutTemplate, Sparkles};
use leptos::prelude::*;

use super::sidenav_routes::{ExplonzRoutes, HooksRoutes, SidenavRoutes, SpotsRoutes};
use crate::components::ui::dropdown_menu::{
    DropdownMenu, DropdownMenuAction, DropdownMenuAlign, DropdownMenuContent, DropdownMenuGroup,
    DropdownMenuItem, DropdownMenuTrigger,
};

#[component]
pub fn SidenavRoutesSelector(
    current_section: Memo<ExplonzRoutes>,
    sidenav_route: SidenavRoutes,
) -> impl IntoView {
    let docs_routes = [ExplonzRoutes::Spots, ExplonzRoutes::Hooks];

    view! {
        <DropdownMenu align=DropdownMenuAlign::Center>
            <DropdownMenuTrigger class="flex justify-between px-2 w-full h-12 bg-transparent border-0">
                <div class="flex gap-2 items-center">
                    <div class="flex justify-center items-center rounded-lg bg-primary text-primary-foreground aspect-square size-8">
                        {move || match current_section.get() {
                            ExplonzRoutes::Spots => view! { <LayoutTemplate /> }.into_any(),
                            ExplonzRoutes::Hooks => view! { <Sparkles /> }.into_any(),
                        }}
                    </div>

                    <div class="grid flex-1 text-sm leading-tight text-left">
                        <span class="font-medium">"Explonz"</span>
                        <span class="text-xs">{move || current_section.get().to_title()}</span>
                    </div>
                </div>

                <ChevronsUpDown />
            </DropdownMenuTrigger>

            <DropdownMenuContent>
                <DropdownMenuGroup>
                    {docs_routes
                        .into_iter()
                        .map(|doc_route| {
                            view! {
                                <DropdownMenuItem>
                                    <DropdownMenuAction href=match doc_route {
                                        ExplonzRoutes::Spots => SpotsRoutes::base_url_with_sidenav(sidenav_route),
                                        ExplonzRoutes::Hooks => HooksRoutes::base_url_with_sidenav(sidenav_route),
                                    }>{doc_route.to_title()}</DropdownMenuAction>
                                </DropdownMenuItem>
                            }
                        })
                        .collect_view()}
                </DropdownMenuGroup>
            </DropdownMenuContent>
        </DropdownMenu>
    }
}

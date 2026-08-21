use leptos::prelude::*;
use leptos_router::{components::{Outlet, ParentRoute, Route}, path};
#[allow(unused_imports)]
use leptos_router::{MatchNestedRoutes, StaticSegment, WildcardSegment};

// use super::sidenav_inset_right::SidenavInsetRight;
use super::sidenav_routes::ExplonzRoutes;
// use crate::components::ui::sidenav::SidenavVariant;
use crate::{components::{blocks::sidenav_inset_right::SidenavInsetRight, ui::sidenav::SidenavVariant}, pages::spots::{addition::SpotAddition, list::SpotList}};

#[component(transparent)]
pub fn SidenavRoutesSimplified(
    #[prop(into, optional)] data_variant: Option<SidenavVariant>,
) -> impl MatchNestedRoutes + Clone {
    view! {
        // * Layout with @sidenav_inset_right
        <ParentRoute
            path=StaticSegment(ExplonzRoutes::base_segment())
            view=move || {
                if let Some(variant) = data_variant {
                    view! { 
                        <SidenavInsetRight data_variant=variant /> 
                        // <div>"variant"</div>
                    }
                } else {
                    view! { 
                        <SidenavInsetRight /> 
                        // <div>"no variant"</div>

                    }
                }
            }
        >
            <Route path=StaticSegment("") view=|| () />

            // Components section - simplified with WildcardSegment
            <ParentRoute path=StaticSegment(ExplonzRoutes::Spots.as_ref()) view=|| view! { <Outlet /> }>
                <Route path=StaticSegment("") view=|| () />
                // <Route path=WildcardSegment("component_path") view=|| () />
                <Route path=path!("/addition") view=SpotAddition/>
                <Route path=path!("/spot_list")     view=SpotList/>
            </ParentRoute>

            // Hooks section - simplified with WildcardSegment
            <ParentRoute path=StaticSegment(ExplonzRoutes::Hooks.as_ref()) view=|| view! { <Outlet /> }>
                <Route path=StaticSegment("") view=|| () />
                <Route path=WildcardSegment("hook_path") view=|| () />
            </ParentRoute>
        </ParentRoute>
    }
    .into_inner()
}

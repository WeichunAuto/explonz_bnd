use leptos::prelude::*;
use leptos_meta::*;
use leptos_router::{
    components::{ParentRoute, Route, Router, Routes},
    path,
};

use crate::pages::{
    auth_guard::AuthGuard, dashboard::Dashboard, home::index::{Sidenav02Routes, SidenavLayout}, login::login::LoginPage, posts::list::PostList, spots::{addition::SpotAddition, list::SpotList},
};

#[component]
pub fn App() -> impl IntoView {
    provide_meta_context();

    view! {
        <Stylesheet id="leptos" href="/pkg/explonz_admin.css"/>
        <Title text="Explonz Admin"/>
        <Router>
            <Routes fallback=|| "404 Not Found">
                <Route path=path!("/login")  view=LoginPage/>

                <ParentRoute path=path!("/") view=AuthGuard>
                    <Sidenav02Routes />
                    <Route path=path!("/dashboard") view=Dashboard/>
                    <Route path=path!("/posts")     view=PostList/>
                    <Route path=path!("/spots")     view=SpotList/>
                    <Route path=path!("/spots/new") view=SpotAddition/>
                </ParentRoute>
            </Routes>
        </Router>
    }
}

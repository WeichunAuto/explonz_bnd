use leptos::prelude::*;
use leptos_meta::*;
use leptos_router::{
    components::{ParentRoute, Route, Router, Routes},
    path,
};

use crate::pages::{auth_guard::AuthGuard, home::index::Sidenav02Routes, login::login::LoginPage};

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
                </ParentRoute>
            </Routes>
        </Router>
    }
}

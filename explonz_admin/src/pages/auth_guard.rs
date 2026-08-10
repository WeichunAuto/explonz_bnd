use leptos::prelude::*;
use leptos::{component, server::Resource, view, IntoView};
use leptos_router::{components::Outlet, hooks::use_navigate, NavigateOptions};
use leptos_ui::clx::{IntoAny, Suspense};

use crate::server::auth::get_current_user;

#[component]
pub fn AuthGuard() -> impl IntoView {
    let user = Resource::new(|| (), |_| get_current_user());
    let navigate = use_navigate();

    view! {
        <Suspense fallback= move || view! { <div class="flex h-screen items-center justify-center">"load..."</div> }>
            {move || {
                match user.get() {
                    // 数据未就绪，Suspense fallback 已处理
                    None => view! { <Outlet/> }.into_any(),
                    // 未登录或 token 失效 → 跳转登录页
                    Some(Ok(None)) | Some(Err(_)) => {
                        navigate("/login", NavigateOptions::default());
                        view! {}.into_any()
                    }
                    // 已登录 → 渲染子路由
                    Some(Ok(Some(_user))) => view! { <Outlet/> }.into_any(),
                }
            }}
        </Suspense>
    }
}

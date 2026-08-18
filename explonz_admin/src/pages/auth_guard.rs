use explonz_shared::common::dto::AuthStatus;
use leptos::prelude::*;
use leptos::{component, server::Resource, view, IntoView};
use leptos_router::components::Outlet;
use leptos_router::hooks::use_location;
use leptos_ui::clx::{IntoAny, Suspense};

#[cfg(not(feature = "ssr"))]
use leptos_router::{hooks::use_navigate, NavigateOptions};

use crate::server::auth::get_current_user;

#[component]
pub fn AuthGuard() -> impl IntoView {
    let location = use_location();
    let user = Resource::new(|| (), |_| get_current_user());

    #[cfg(not(feature = "ssr"))]
    let navigate = use_navigate();

    view! {
        <Suspense fallback= move || view! { <div class="flex h-screen items-center justify-center">"load..."</div> }>
            {move || {
                match user.get() {
                    // 数据未就绪，Suspense fallback 已处理
                    None => view! { <Outlet/> }.into_any(),

                    // 已登录 → 渲染子路由
                    Some(Ok(AuthStatus::Authenticated(_))) => view! { <Outlet/> }.into_any(),

                    // 无 Cookie（从未登录）→ reason=login_required
                    Some(Ok(AuthStatus::NotLoggedIn)) => {
                        let pathname = location.pathname.get_untracked();
                        let url = format!("/login?redirect={}&reason={}", pathname, AuthStatus::NotLoggedIn);

                        #[cfg(feature = "ssr")]
                        leptos_axum::redirect(&url);

                        #[cfg(not(feature = "ssr"))]
                        navigate(&url, NavigateOptions::default());

                        view! {}.into_any()
                    }

                    // Cookie 存在但 JWT 无效/过期 → reason=session_expired
                    Some(Ok(AuthStatus::TokenExpired)) => {
                        let pathname = location.pathname.get_untracked();
                        let url = format!("/login?redirect={}&reason={}", pathname, AuthStatus::TokenExpired);

                        #[cfg(feature = "ssr")]
                        leptos_axum::redirect(&url);

                        #[cfg(not(feature = "ssr"))]
                        navigate(&url, NavigateOptions::default());

                        view! {}.into_any()
                    }

                    // Server Function 本身调用出错 → 安全起见按未登录处理
                    Some(Err(_)) => {
                        let url = format!("/login?reason={}", AuthStatus::TokenExpired);
                        #[cfg(feature = "ssr")]
                        leptos_axum::redirect(&url);

                        #[cfg(not(feature = "ssr"))]
                        navigate(&url, NavigateOptions::default());

                        view! {}.into_any()
                    }
                }
            }}
        </Suspense>
    }
}

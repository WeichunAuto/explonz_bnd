#[cfg(feature = "ssr")]
#[tokio::main]
async fn main() {
    use axum::Router;
    use explonz_admin::app::App;
    use leptos::prelude::*;
    use leptos_axum::{generate_route_list, LeptosRoutes};

    // dotenvy::dotenv() 从 CWD 查找 .env，cargo leptos 在 workspace 根运行，
    // 会加载 explonz_bnd/.env 而非 explonz_admin/.env。
    // 用 CARGO_MANIFEST_DIR（编译期绝对路径）确保始终加载 explonz_admin/.env。
    dotenvy::from_path(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(".env")
    ).ok();

    let conf = get_configuration(None).unwrap();
    let leptos_options = conf.leptos_options;
    let addr = leptos_options.site_addr;
    let routes = generate_route_list(App);

    let app = Router::new()
        .leptos_routes(&leptos_options, routes, {
            let leptos_options = leptos_options.clone();
            move || shell(leptos_options.clone())
        })
        .fallback(leptos_axum::file_and_error_handler(shell))
        .with_state(leptos_options);

    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    tracing::info!("explonz_admin listening on {}", addr);
    axum::serve(listener, app).await.unwrap();
}

#[cfg(feature = "ssr")]
fn shell(options: leptos::prelude::LeptosOptions) -> impl leptos::prelude::IntoView {
    use explonz_admin::app::App;
    use leptos::hydration::{AutoReload, HydrationScripts};
    use leptos::prelude::*;
    use leptos_meta::*;

    view! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8"/>
                <meta name="viewport" content="width=device-width, initial-scale=1"/>
                <AutoReload options=options.clone()/>
                <HydrationScripts options/>
                <MetaTags/>
            </head>
            <body>
                <App/>
            </body>
        </html>
    }
}

#[cfg(not(feature = "ssr"))]
pub fn main() {}

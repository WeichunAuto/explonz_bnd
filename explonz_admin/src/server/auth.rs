use explonz_shared::common::dto::AdminUser;
use leptos::server;
use leptos_ui::clx::ServerFnError;

#[server]
pub async fn get_current_user() -> Result<Option<AdminUser>, ServerFnError> {
    Ok(None)
}

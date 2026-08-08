use explonz_shared::common::dto::PostDto;
use leptos::prelude::*;

#[server(GetPosts, "/api")]
pub async fn get_posts(page: u64, page_size: u64) -> Result<Vec<PostDto>, ServerFnError> {
    use explonz_shared::entity::posts;
    use sea_orm::{DatabaseConnection, EntityTrait};

    let db = use_context::<DatabaseConnection>()
        .ok_or_else(|| ServerFnError::new("No DB connection"))?;
    let models = posts::Entity::find()
        .all(&db)
        .await
        .map_err(|e| ServerFnError::new(e))?;
    Ok(models.into_iter().map(PostDto::from).collect())
}

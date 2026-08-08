use explonz_shared::common::dto::SpotDto;
use leptos::prelude::*;

#[server(GetSpots, "/api")]
pub async fn get_spots(page: u64, page_size: u64) -> Result<Vec<SpotDto>, ServerFnError> {
    use explonz_shared::entity::spots;
    use sea_orm::{DatabaseConnection, EntityTrait};

    let db = use_context::<DatabaseConnection>()
        .ok_or_else(|| ServerFnError::new("No DB connection"))?;
    let models = spots::Entity::find()
        .all(&db)
        .await
        .map_err(|e| ServerFnError::new(e))?;
    Ok(models.into_iter().map(SpotDto::from).collect())
}

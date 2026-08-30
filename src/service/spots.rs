use explonz_shared::common::dto::SpotDto;
use sea_orm::ActiveModelTrait;
use sea_orm::{ActiveValue::Set, DatabaseConnection};

use crate::api::spots::dto::CreateSpotRequest;
use explonz_shared::entity::{prelude::*, spots};

pub async fn create_spot_service(
    db: &DatabaseConnection,
    spot_request: CreateSpotRequest,
) -> anyhow::Result<SpotDto> {
    let model = spots::ActiveModel {
        name: Set(spot_request.name.clone()),
        location: Set(spot_request.location),
        latitude: Set(spot_request.latitude),
        longitude: Set(spot_request.longitude),
        description: Set(spot_request.description),
        photo_urls: Set(spot_request.photo_urls),
        attributes: Set(spot_request.attributes),
        phone: Set(spot_request.phone),
        website: Set(spot_request.website),

        ..Default::default() // id 由 uuidv7() 数据库生成，rating 默认 0.0
    };
    let result = model.insert(db).await?;

    tracing::info!("spot has been created, spot name: {}", spot_request.name);
    // crate::entity::spots::Model 与 explonz_shared::entity::spots::Model 为不同类型，
    // 无法使用 SpotDto::from()，手动构造
    Ok(SpotDto {
        id: result.id,
        name: result.name,
        rating: result.rating,
        location: result.location,
        latitude: result.latitude,
        longitude: result.longitude,
        description: result.description,
        photo_urls: result.photo_urls,
        attributes: result.attributes,
        created_at: result.created_at.into(),
        updated_at: result.updated_at.into(),
        phone: result.phone,
        website: result.website,
    })
}

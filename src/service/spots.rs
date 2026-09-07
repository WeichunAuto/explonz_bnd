use explonz_shared::common::dto::SpotDto;
use sea_orm::ActiveModelTrait;
use sea_orm::{ActiveValue::Set, DatabaseConnection, EntityTrait, TransactionTrait};
use uuid::Uuid;

use crate::api::spots::dto::CreateSpotRequest;
use explonz_shared::entity::{prelude::*, spot_label_assignments, spot_opening_hours, spots};

pub async fn create_spot_service(
    db: &DatabaseConnection,
    spot_request: CreateSpotRequest,
) -> anyhow::Result<SpotDto> {
    let txn = db.begin().await?;

    // 1. 插入 spot
    let model = spots::ActiveModel {
        name: Set(spot_request.name.clone()),
        location: Set(spot_request.location),
        latitude: Set(spot_request.latitude),
        longitude: Set(spot_request.longitude),
        description: Set(spot_request.description),
        photo_urls: Set(spot_request.photo_urls),
        attributes: Set(serde_json::json!([])),
        phone: Set(spot_request.phone),
        website: Set(spot_request.website),
        ..Default::default() // id 由 uuidv7() 数据库生成，rating 默认 0.0
    };
    let result = model.insert(&txn).await?;

    // 2. 批量插入 spot_label_assignments
    let assignments: Vec<spot_label_assignments::ActiveModel> = spot_request
        .label_ids
        .iter()
        .filter_map(|id| id.parse::<Uuid>().ok())
        .map(|label_id| spot_label_assignments::ActiveModel {
            spot_id: Set(result.id),
            label_id: Set(label_id),
        })
        .collect();

    if !assignments.is_empty() {
        SpotLabelAssignments::insert_many(assignments)
            .exec(&txn)
            .await?;
    }

    // 3. 批量插入 spot_opening_hours
    let opening_hours: Vec<spot_opening_hours::ActiveModel> = spot_request
        .opening_hours
        .into_iter()
        .map(|h| {
            let parse_time = |s: Option<String>| {
                s.and_then(|t| chrono::NaiveTime::parse_from_str(&t, "%H:%M").ok())
            };
            spot_opening_hours::ActiveModel {
                spot_id: Set(result.id),
                day_of_week: Set(h.day_of_week),
                is_closed: Set(h.is_closed),
                is_open_24h: Set(h.is_open_24h),
                open_time: Set(parse_time(h.open_time)),
                close_time: Set(parse_time(h.close_time)),
                ..Default::default() // id 由 uuidv7() 数据库生成
            }
        })
        .collect();

    if !opening_hours.is_empty() {
        SpotOpeningHours::insert_many(opening_hours)
            .exec(&txn)
            .await?;
    }

    txn.commit().await?;

    tracing::info!("spot has been created, spot name: {}", spot_request.name);

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

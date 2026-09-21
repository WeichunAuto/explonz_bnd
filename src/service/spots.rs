use std::collections::HashMap;

use explonz_shared::common::dto::{
    LabelDto, OpeningHourDto, SeasonalPickingTypeDto, SeasonalPickingsDto, SpotDto,
};
use explonz_shared::common::pagination::Page;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, PaginatorTrait, QueryFilter, QueryOrder, QuerySelect,
};
use sea_orm::{ActiveValue::Set, DatabaseConnection, EntityTrait, TransactionTrait};
use uuid::Uuid;

use crate::api::spots::dto::{CreateSpotRequest, UpdateSpotRequest};
use crate::api::spots::handler::SpotQuery;
use explonz_shared::entity::{
    prelude::*, seasonal_pickings, spot_label_assignments, spot_labels, spot_opening_hours, spots,
};

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
        created_at: result.created_at.into(),
        updated_at: result.updated_at.into(),
        phone: result.phone,
        website: result.website,
        labels: vec![],
        opening_hours: vec![],
    })
}

pub async fn update_spot_service(
    db: &DatabaseConnection,
    spot_id: Uuid,
    req: UpdateSpotRequest,
) -> anyhow::Result<SpotDto> {
    let txn = db.begin().await?;

    // 1. 找到 spot
    let spot = Spots::find_by_id(spot_id)
        .one(&txn)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Spot not found"))?;

    // 2. 更新 spot 字段
    let mut active: spots::ActiveModel = spot.into();
    active.name = Set(req.name.clone());
    active.location = Set(req.location);
    active.latitude = Set(req.latitude);
    active.longitude = Set(req.longitude);
    active.description = Set(req.description);
    active.photo_urls = Set(req.photo_urls);
    active.phone = Set(req.phone);
    active.website = Set(req.website);
    let result = active.update(&txn).await?;

    // 3. 删除旧 label 关联，重新插入
    SpotLabelAssignments::delete_many()
        .filter(spot_label_assignments::Column::SpotId.eq(spot_id))
        .exec(&txn)
        .await?;

    let assignments: Vec<spot_label_assignments::ActiveModel> = req
        .label_ids
        .iter()
        .filter_map(|id| id.parse::<Uuid>().ok())
        .map(|label_id| spot_label_assignments::ActiveModel {
            spot_id: Set(spot_id),
            label_id: Set(label_id),
        })
        .collect();
    if !assignments.is_empty() {
        SpotLabelAssignments::insert_many(assignments)
            .exec(&txn)
            .await?;
    }

    // 4. 删除旧营业时间，重新插入
    SpotOpeningHours::delete_many()
        .filter(spot_opening_hours::Column::SpotId.eq(spot_id))
        .exec(&txn)
        .await?;

    let opening_hours: Vec<spot_opening_hours::ActiveModel> = req
        .opening_hours
        .into_iter()
        .map(|h| {
            let parse_time = |s: Option<String>| {
                s.and_then(|t| chrono::NaiveTime::parse_from_str(&t, "%H:%M").ok())
            };
            spot_opening_hours::ActiveModel {
                spot_id: Set(spot_id),
                day_of_week: Set(h.day_of_week),
                is_closed: Set(h.is_closed),
                is_open_24h: Set(h.is_open_24h),
                open_time: Set(parse_time(h.open_time)),
                close_time: Set(parse_time(h.close_time)),
                ..Default::default()
            }
        })
        .collect();
    if !opening_hours.is_empty() {
        SpotOpeningHours::insert_many(opening_hours)
            .exec(&txn)
            .await?;
    }

    txn.commit().await?;

    tracing::info!("spot has been updated, spot name: {}", req.name);

    let (mut labels_map, mut hours_map) = load_labels_and_hours(db, &[spot_id]).await;
    let mut dto = SpotDto::from(result);
    dto.labels = labels_map.remove(&spot_id).unwrap_or_default();
    dto.opening_hours = hours_map.remove(&spot_id).unwrap_or_default();
    Ok(dto)
}

// 获取所有分页 Spots（含 labels + opening_hours）
pub async fn get_spots_service(
    db: &DatabaseConnection,
    public_url: &str,
    spot_params: SpotQuery,
) -> Page<SpotDto> {
    let mut query = Spots::find();
    if let Some(spot_id) = spot_params.id {
        query = query.filter(spots::Column::Id.eq(spot_id))
    }
    if let Some(spot_name) = spot_params.name {
        query = query.filter(spots::Column::Name.contains(spot_name))
    }
    query = query.order_by_desc(spots::Column::CreatedAt);

    let pagination = spot_params.pagination;
    let paginator = query.paginate(db, pagination.size);
    let total = paginator.num_items().await.unwrap_or_else(|_| {
        tracing::error!("error getting total");
        0
    });
    let items = paginator
        .fetch_page(pagination.page - 1)
        .await
        .unwrap_or_else(|_| {
            tracing::error!("error fetching page");
            vec![]
        });

    let spot_ids: Vec<Uuid> = items.iter().map(|m| m.id).collect();

    let (labels_map, hours_map) = if spot_ids.is_empty() {
        (HashMap::new(), HashMap::new())
    } else {
        load_labels_and_hours(db, &spot_ids).await
    };

    let spots: Vec<SpotDto> = items
        .into_iter()
        .map(|m| {
            let id = m.id;
            let mut dto = SpotDto::from(m);

            // 修正 spot 的图片地址, 拼接服务器地址
            let trimed_urls: Vec<String> = dto
                .photo_urls
                .iter()
                .map(|url| {
                    format!(
                        "{}{}",
                        public_url,
                        url.find("/uploads/")
                            .map(|i| &url[i..])
                            .unwrap_or_default()
                            .to_string()
                    )
                })
                .collect();
            dto.photo_urls = trimed_urls;

            dto.labels = labels_map.get(&id).cloned().unwrap_or_default();
            dto.opening_hours = hours_map.get(&id).cloned().unwrap_or_default();
            dto
        })
        .collect();

    Page::from_pagination(&pagination, total, spots)
}

// 根据 spot_id 获取某个 spot 详情（含 labels + opening_hours）
pub async fn get_spot_service(
    db: &DatabaseConnection,
    spot_id: Uuid,
) -> anyhow::Result<Option<SpotDto>> {
    let model = Spots::find_by_id(spot_id).one(db).await?;
    let Some(model) = model else {
        return Ok(None);
    };
    let (mut labels_map, mut hours_map) = load_labels_and_hours(db, &[spot_id]).await;
    let mut dto = SpotDto::from(model);
    dto.labels = labels_map.remove(&spot_id).unwrap_or_default();
    dto.opening_hours = hours_map.remove(&spot_id).unwrap_or_default();
    Ok(Some(dto))
}

// 获取所有的 seasonal_picking_types
pub async fn get_seasonal_picking_types_service(
    db: &DatabaseConnection,
) -> anyhow::Result<Vec<SeasonalPickingTypeDto>> {
    let model = SeasonalPickingTypes::find().all(db).await?;

    let seasonal_picking_types = model
        .iter()
        .map(|item| SeasonalPickingTypeDto {
            id: item.id,
            name: item.name.clone(),
        })
        .collect::<Vec<SeasonalPickingTypeDto>>();
    // tracing::info!("seasonal_picking_types: {:?}", seasonal_picking_types);
    Ok(seasonal_picking_types)
}

/// 批量加载 labels 和 opening_hours，返回两个 HashMap (spot_id -> Vec)
async fn load_labels_and_hours(
    db: &DatabaseConnection,
    spot_ids: &[Uuid],
) -> (
    HashMap<Uuid, Vec<LabelDto>>,
    HashMap<Uuid, Vec<OpeningHourDto>>,
) {
    // 1. labels: spot_label_assignments JOIN spot_labels
    let assignments: Vec<(spot_label_assignments::Model, Option<spot_labels::Model>)> =
        SpotLabelAssignments::find()
            .filter(spot_label_assignments::Column::SpotId.is_in(spot_ids.to_vec()))
            .find_also_related(spot_labels::Entity)
            .all(db)
            .await
            .unwrap_or_default();

    let mut labels_map: HashMap<Uuid, Vec<LabelDto>> = HashMap::new();
    for (assignment, label_opt) in assignments {
        if let Some(label) = label_opt {
            labels_map
                .entry(assignment.spot_id)
                .or_default()
                .push(LabelDto {
                    id: label.id,
                    name: label.name,
                    description: label.description,
                    icon: label.icon,
                });
        }
    }

    // 2. opening_hours
    let all_hours: Vec<spot_opening_hours::Model> = SpotOpeningHours::find()
        .filter(spot_opening_hours::Column::SpotId.is_in(spot_ids.to_vec()))
        .order_by_asc(spot_opening_hours::Column::DayOfWeek)
        .all(db)
        .await
        .unwrap_or_default();

    let mut hours_map: HashMap<Uuid, Vec<OpeningHourDto>> = HashMap::new();
    for h in all_hours {
        let dto = OpeningHourDto {
            day_of_week: h.day_of_week,
            is_closed: h.is_closed,
            is_open_24h: h.is_open_24h,
            open_time: h.open_time.map(|t| t.format("%H:%M").to_string()),
            close_time: h.close_time.map(|t| t.format("%H:%M").to_string()),
        };
        hours_map.entry(h.spot_id).or_default().push(dto);
    }

    (labels_map, hours_map)
}

// 为 Spot 创建一条 SeasonalPicking
pub async fn create_seasonal_picking_service(
    db: &DatabaseConnection,
    spot_id: Uuid,
    seasonal_pickings_params: SeasonalPickingsDto,
) -> anyhow::Result<()> {
    tracing::info!("payload: {:?}", seasonal_pickings_params);
    let model = seasonal_pickings::ActiveModel {
        spot_id: Set(spot_id),
        season_start_month: Set(seasonal_pickings_params.start_month),
        season_start_day: Set(seasonal_pickings_params.start_day),
        season_end_month: Set(seasonal_pickings_params.end_month),
        season_end_day: Set(seasonal_pickings_params.end_day),
        picking_type_id: Set(Uuid::parse_str(seasonal_pickings_params.type_id.as_ref())
            .map_err(|e| format!("Invalid UUID: {e}"))
            .unwrap()),
        ..Default::default()
    };
    model.insert(db).await?;
    Ok(())
}

use explonz_shared::{common::dto::LabelDto, entity::spot_labels};
use sea_orm::{ActiveModelTrait, ActiveValue::Set, DatabaseConnection, EntityTrait};
use uuid::Uuid;

use crate::api::labels::dto::CreateLabelRequest;

// 查询所有 Labels
pub async fn get_labels_service(db: &DatabaseConnection) -> anyhow::Result<Vec<LabelDto>> {
    let models = spot_labels::Entity::find().all(db).await?;
    Ok(models
        .into_iter()
        .map(|m| LabelDto {
            id: m.id.into(),
            name: m.name,
            description: m.description,
            icon: m.icon,
        })
        .collect())
}

// 创建 Label
pub async fn create_label_service(
    db: &DatabaseConnection,
    label_request: CreateLabelRequest,
) -> anyhow::Result<LabelDto> {
    let model = spot_labels::ActiveModel {
        name: Set(label_request.name),
        description: Set(label_request.description),
        icon: Set(label_request.icon),
        ..Default::default()
    };
    let result = model.insert(db).await?;

    Ok(LabelDto {
        id: result.id.into(),
        name: result.name,
        description: result.description,
        icon: result.icon,
    })
}

// 删除 Label
pub async fn delete_label_service(db: &DatabaseConnection, id: String) -> anyhow::Result<()> {
    spot_labels::Entity::delete_by_id(id.parse::<Uuid>().unwrap())
        .exec(db)
        .await?;
    Ok(())
}

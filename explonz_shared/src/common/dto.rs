use chrono::{DateTime, FixedOffset};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PostType {
    Standard,
    Comment,
    Repost,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PostDto {
    pub id: Uuid,
    pub post_type: PostType,
    pub author_id: Uuid,
    pub title: String,
    pub body: String,
    pub spot_id: Option<Uuid>,
    pub original_post_id: Option<Uuid>,
    pub like_count: i32,
    pub comment_count: i32,
    pub repost_count: i32,
    pub created_at: DateTime<FixedOffset>,
    pub updated_at: DateTime<FixedOffset>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpotDto {
    pub id: Uuid,
    pub name: String,
    pub rating: Decimal,
    pub location: String,
    pub latitude: f64,
    pub longitude: f64,
    pub description: String,
    pub photo_urls: Vec<String>,
    pub attributes: Value,
    pub created_at: DateTime<FixedOffset>,
    pub updated_at: DateTime<FixedOffset>,
}

#[cfg(feature = "ssr")]
impl From<crate::entity::posts::Model> for PostDto {
    fn from(m: crate::entity::posts::Model) -> Self {
        use crate::entity::sea_orm_active_enums::PostTypeEnum;
        Self {
            id: m.id,
            post_type: match m.r#type {
                PostTypeEnum::Standard => PostType::Standard,
                PostTypeEnum::Comment => PostType::Comment,
                PostTypeEnum::Repost => PostType::Repost,
            },
            author_id: m.author_id,
            title: m.title,
            body: m.body,
            spot_id: m.spot_id,
            original_post_id: m.original_post_id,
            like_count: m.like_count,
            comment_count: m.comment_count,
            repost_count: m.repost_count,
            created_at: m.created_at.into(),
            updated_at: m.updated_at.into(),
        }
    }
}

#[cfg(feature = "ssr")]
impl From<crate::entity::spots::Model> for SpotDto {
    fn from(m: crate::entity::spots::Model) -> Self {
        Self {
            id: m.id,
            name: m.name,
            rating: m.rating,
            location: m.location,
            latitude: m.latitude,
            longitude: m.longitude,
            description: m.description,
            photo_urls: m.photo_urls,
            attributes: m.attributes,
            created_at: m.created_at.into(),
            updated_at: m.updated_at.into(),
        }
    }
}

// use crate::application::AppState;
// use crate::common::{Page, Pagination};
// use crate::entity::prelude::*;
// use crate::entity::users;
// use crate::entity::users::{ActiveModel, Model};
// use crate::request::BValidQuery;
// use crate::response::ApiResponse;
// use axum::extract::Path;
// use axum::extract::State;
// use axum::Json;
// use sea_orm::{prelude::*, Condition, QueryOrder, Set};
// use serde::Deserialize;
// use std::fmt::{Display, Formatter};
// use validator::Validate;

// #[derive(Debug, Deserialize, Validate, DeriveIntoActiveModel)]
// pub(crate) struct CreateUserRequest {
//     #[validate(length(
//         min = 1,
//         max = 14,
//         message = "fullname must be between 1 and 14 characters"
//     ))]
//     pub fullname: String,
//     pub gender: Option<Gender>,
//     #[validate(custom(
//         function = "crate::request::is_email_valid",
//         message = "invalid email format, please check."
//     ))]
//     pub email: String,
//     pub password_hash: String,
//     #[validate(range(min = 1, message = "ws_id must be greater than 0"))]
//     pub ws_id: i64,
// }

// #[derive(Debug, Deserialize, Validate)]
// pub(crate) struct UserQuery {
//     pub keyword: Option<String>,
//     pub id: Option<u64>,
//     pub name: Option<String>,
//     #[validate(nested)]
//     #[serde(flatten)]
//     // Flatten the nested Pagination struct fields into the current struct to avoid nested levels in JSON.
//     pub pagination: Option<Pagination>,
// }

// /// delete user by id
// // #[tracing::instrument(name = "delete_user_by_id", skip(db))]
// // pub(crate) async fn delete_by_id(
// //     State(AppState { db }): State<AppState>,
// //     Path(id): Path<u64>,
// // ) -> ApiResponse<()> {
// //     let rt = users::Entity::delete_by_id(id as i64).exec(&db).await;

// //     match rt {
// //         Ok(deleted_user) => {
// //             if deleted_user.rows_affected > 0 {
// //                 tracing::info!("User was deleted successfully with id = : {:?}!", id);
// //                 ApiResponse::success("User was deleted successfully!", None)
// //             } else {
// //                 tracing::error!("When delete the user, with id = : {:?} not found", id);
// //                 ApiResponse::error(format!("User with id = : {:?} not found", id))
// //             }
// //         }
// //         Err(e) => {
// //             tracing::error!("error deleting user: {:?}", e);
// //             ApiResponse::error(format!("error deleting user: {:?}", e))
// //         }
// //     }
// // }

// // /// update user ws_id by id
// // #[tracing::instrument(name = "update_ws_by_id", skip(state))]
// // pub(crate) async fn update_ws_by_id(
// //     State(state): State<AppState>,
// //     Path((id, ws_id)): Path<(u64, u64)>,
// // ) -> ApiResponse<Model> {
// //     let db = state.db();

// //     let rt = users::Entity::update(users::ActiveModel {
// //         id: Set(id as i64),
// //         ws_id: Set(ws_id as i64),
// //         ..Default::default()
// //     })
// //     .exec(db)
// //     .await;

// //     match rt {
// //         Ok(user) => {
// //             tracing::info!(
// //                 "user updated successfully with id = : {:?}, name = : {:?}",
// //                 user.id,
// //                 user.fullname
// //             );
// //             ApiResponse::success("User updated successfully!", Some(user))
// //         }
// //         Err(DbErr::RecordNotUpdated) => {
// //             tracing::error!("User id: {} not found", id);
// //             ApiResponse::error(format!("User id: {} not found", id))
// //         }
// //         Err(e) => {
// //             tracing::error!("error updating user: {:?}", e);
// //             ApiResponse::error(format!("error updating user: {:?}", e))
// //         }
// //     }
// // }

// /// create user
// #[tracing::instrument(name="create_user", skip(state), fields(user_data = %user_data))]
// pub(crate) async fn create(
//     State(state): State<AppState>,
//     Json(user_data): Json<CreateUserRequest>,
// ) -> ApiResponse<Model> {
//     if let Err(ret) = user_data.validate() {
//         tracing::error!("error validating user: {:?}", ret);
//         return ApiResponse::error(format!("error validating user: {:?}", ret.to_string()));
//     }

//     let db = state.db();

//     let existing_user = Users::find()
//         .filter(users::Column::Email.eq(&user_data.email))
//         .one(db)
//         .await
//         .unwrap();

//     if existing_user.is_some() {
//         tracing::warn!("user with email {} already exists", &user_data.email);
//         return ApiResponse::error(format!(
//             "user with email ({}) already exists",
//             &user_data.email
//         ));
//     }

//     let new_user = ActiveModel {
//         fullname: Set(user_data.fullname),
//         gender: Set(user_data.gender),
//         email: Set(user_data.email),
//         password_hash: Set(user_data.password_hash),
//         ws_id: Set(user_data.ws_id),
//         ..Default::default()
//     };

//     let rt = new_user.insert(db).await;

//     match rt {
//         Ok(user) => {
//             tracing::info!(
//                 "user created successfully with id = : {:?} and name = : {:?}",
//                 user.id,
//                 user.fullname
//             );
//             ApiResponse::success("User created successfully!", Some(user))
//         }
//         Err(e) => {
//             tracing::error!("error creating user: {:?}", e);
//             ApiResponse::error(format!("error creating user: {:?}", e))
//         }
//     }
// }

// // /// query all users by id and name
// // #[tracing::instrument(name="query_all_by_id_or_name", skip(state), fields(UserQuery = %params))]
// // pub(crate) async fn query_all_by_id_or_name(
// //     State(state): State<AppState>,
// //     BValidQuery(params): BValidQuery<UserQuery>,
// // ) -> ApiResponse<Vec<Model>> {
// //     let db = state.db();

// //         let mut conditions = Condition::all();

// //     if let Some(id) = params.id {
// //         conditions = conditions.add(users::Column::Id.eq(id));
// //     }
// //     if let Some(name) = params.name {
// //         conditions = conditions.add(users::Column::Fullname.eq(name));
// //     }

// //     let users = Users::find()
// //         .filter(conditions)
// //         .order_by_desc(users::Column::CreateAt)
// //         .all(db)
// //         .await
// //         .unwrap();
// //     tracing::info!("query users results: {:?}", users);
// //     ApiResponse::success("success", Some(users))
// // }

// // // #[debug_handler]
// // pub async fn query_by_keyword(
// //     State(AppState { db }): State<AppState>,
// //     BValidQuery(params): BValidQuery<UserQuery>, // apply validator
// // ) -> ApiResponse<Page<Model>> {
// //     let mut query = Users::find();

// //     if let Some(keyword) = params.keyword.as_ref() {
// //         query = query.filter(
// //             Condition::any()
// //                 .add(users::Column::Fullname.contains(keyword))
// //                 .add(users::Column::Email.contains(keyword)),
// //         );
// //     }
// //     query = query.order_by_desc(users::Column::CreateAt);

// //     let (pagination, items, total) = if let Some(pagination) = params.pagination {
// //         let pagination = Pagination {
// //             page: pagination.page,
// //             size: pagination.size,
// //         };
// //         let paginator = query.paginate(&db, pagination.size);
// //         let total = paginator.num_items().await.unwrap_or_else(|_| {
// //             tracing::error!("error getting total");
// //             0
// //         });
// //         let items = paginator
// //             .fetch_page(pagination.page - 1)
// //             .await
// //             .unwrap_or_else(|_| {
// //                 tracing::error!("error getting total");
// //                 vec![]
// //             });

// //         (pagination, items, total)
// //     } else {
// //         let items = query.all(&db).await.unwrap_or_else(|_| {
// //             tracing::error!("error getting total");
// //             vec![]
// //         });
// //         let total = items.len() as u64;
// //         let pagination = Pagination {
// //             page: 1,
// //             size: total,
// //         };
// //         (pagination, items, total)
// //     };

// //     let page = Page::from_pagination(&pagination, total, items);

// //     ApiResponse::success("success", Some(page))
// // }

// impl Display for CreateUserRequest {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         write!(
//             f,
//             "CreateUserRequest {{ fullname: {}, email: {}, password: ****, ws_id: {} }}",
//             self.fullname, self.email, self.ws_id
//         )
//     }
// }

// impl Display for UserQuery {
//     fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
//         write!(f, "UserQuery {{ id: {:?}, name: {:?}}}", self.id, self.name)
//     }
// }

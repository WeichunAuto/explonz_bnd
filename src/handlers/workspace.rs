// use crate::application::AppState;
// use crate::entity::prelude::Workspace;
// use crate::entity::workspace;
// use crate::entity::workspace::ActiveModel;
// use crate::entity::workspace::Model;
// use crate::response::ApiResponse;
// use axum::extract::State;
// use axum::Json;
// use sea_orm::ColumnTrait;
// use sea_orm::{ActiveModelTrait, EntityTrait, QueryFilter, Set};
// use serde::Deserialize;
// use std::fmt::Display;

// #[derive(Deserialize)]
// pub struct CreateWorkspaceRequest {
//     name: String,
//     owner_id: u64,
// }

// /// create workspace
// #[tracing::instrument(name="create_workspace", skip(state), fields(workspace_data = %workspace_data))]
// pub(crate) async fn create_workspace(
//     State(state): State<AppState>,
//     Json(workspace_data): Json<CreateWorkspaceRequest>,
// ) -> ApiResponse<Model> {
//     let db = state.db();

//     let name_exists = Workspace::find()
//         .filter(workspace::Column::Name.eq(&workspace_data.name))
//         .one(db)
//         .await
//         .unwrap();
//     if name_exists.is_some() {
//         tracing::warn!(
//             "workspace with name {} already exists",
//             &workspace_data.name
//         );
//         return ApiResponse::error(format!(
//             "workspace with name ({}) already exists",
//             &workspace_data.name
//         ));
//     }

//     let new_workspace = ActiveModel {
//         name: Set(workspace_data.name),
//         owner_id: Set(workspace_data.owner_id as i64),
//         ..Default::default()
//     };

//     let rt = new_workspace.insert(db).await;
//     match rt {
//         Ok(workspace) => {
//             tracing::info!(
//                 "workspace created successfully with id = : {:?} and name = : {:?}",
//                 workspace.id,
//                 workspace.name
//             );
//             ApiResponse::success("Workspace created successfully!", Some(workspace))
//         }
//         Err(e) => {
//             tracing::error!("error creating workspace: {:?}", e);
//             ApiResponse::error(format!("error creating workspace: {:?}", e))
//         }
//     }
// }

// impl Display for CreateWorkspaceRequest {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         write!(
//             f,
//             "CreateWorkspaceRequest {{ name: {}, owner_id: {} }}",
//             self.name, self.owner_id
//         )
//     }
// }

use crate::api::auth::User;
use crate::app::{self, WelcomeEmail, WelcomeEmailJob};

use axum::{
    extract::Path,
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Json},
    Extension,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::PgPool;
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;
use uuid::Uuid;

pub fn router() -> OpenApiRouter<app::State> {
    OpenApiRouter::new()
        .routes(routes!(create_campaign, get_campaigns))
        .routes(routes!(get_campaign, update_campaign, delete_campaign))
        .routes(routes!(start_campaign))
}

#[utoipa::path(
  post,
  path = "",
  request_body = CreateCampaignInput,
  responses(
    (status = NO_CONTENT)
  )
)]
async fn create_campaign(
    State(db): State<PgPool>,
    Extension(user): Extension<User>,
    Json(input): Json<CreateCampaignInput>,
) -> impl IntoResponse {
    let campaign_id = CampaignId::new();

    // TODO: handle errors here
    let _ = sqlx::query("insert into campaign (id, name, user_id) values ($1, $2, $3)")
        .bind(&campaign_id)
        .bind(&input.name)
        .bind(&user.id)
        .execute(&db)
        .await;

    Json(campaign_id)
}

#[utoipa::path(
  get,
  path = "",
  responses((status = OK, body = Vec<Campaign>)))]
async fn get_campaigns(
    State(db): State<PgPool>,
    Extension(user): Extension<User>,
) -> impl IntoResponse {
    // TODO: handle errors here
    let campaigns = sqlx::query_as::<_, Campaign>("select * from campaign where user_id = $1")
        .bind(&user.id)
        .fetch_all(&db)
        .await
        .unwrap();

    Json(campaigns)
}

#[utoipa::path(
  get,
  path = "/{campaign_id}",
  params(
    ("campaign_id" = CampaignId, Path)
  ),
  responses(
    (status = OK, body = Campaign)
  )
)]
async fn get_campaign(
    State(db): State<PgPool>,
    Path(campaign_id): Path<CampaignId>,
    Extension(user): Extension<User>,
) -> impl IntoResponse {
    // TODO: handle errors here
    let campaign =
        sqlx::query_as::<_, Campaign>("select * from campaign where id = $1 and user_id = $2")
            .bind(&campaign_id)
            .bind(&user.id)
            .fetch_optional(&db)
            .await
            .unwrap();

    Json(campaign)
}

#[utoipa::path(
  patch,
  path = "/{campaign_id}",
  params(
    ("campaign_id" = CampaignId, Path)
  ),
  request_body = UpdateCampaignInput,
  responses(
    (status = NO_CONTENT)
  )
)]
async fn update_campaign(
    State(db): State<PgPool>,
    Path(campaign_id): Path<CampaignId>,
    Extension(user): Extension<User>,
    Json(input): Json<UpdateCampaignInput>,
) -> impl IntoResponse {
    // TODO: handle errors here
    let _ = sqlx::query(
        "update campaign
         set name = $1,
             email_source = $2,
             email_content = $3
         where id = $4 and user_id = $5",
    )
    .bind(&input.name)
    .bind(&input.email_source)
    .bind(&input.email_content)
    .bind(&campaign_id)
    .bind(&user.id)
    .execute(&db)
    .await
    .unwrap();

    StatusCode::NO_CONTENT
}

#[utoipa::path(
  delete,
  path = "/{campaign_id}",
  params(
    ("campaign_id" = CampaignId, Path)
  ),
  responses(
    (status = NO_CONTENT)
  )
)]
async fn delete_campaign(
    State(db): State<PgPool>,
    Path(campaign_id): Path<CampaignId>,
    Extension(user): Extension<User>,
) -> impl IntoResponse {
    // TODO: handle errors here
    let _ = sqlx::query("delete from campaign where id = $1 and user_id = $2")
        .bind(&campaign_id)
        .bind(&user.id)
        .execute(&db)
        .await
        .unwrap();

    StatusCode::NO_CONTENT
}

#[utoipa::path(
  post,
  path = "/{campaign_id}/start",
    params(
    ("campaign_id" = CampaignId, Path)
  ),
  responses(
    (status = NO_CONTENT)
  )
)]
async fn start_campaign(
    State(db): State<PgPool>,
    State(job): State<WelcomeEmailJob>,
    Path(campaign_id): Path<CampaignId>,
    Extension(user): Extension<User>,
) -> impl IntoResponse {
    // Enqueue a job task.
    let task_id = job
        .enqueue(WelcomeEmail {
            user_id: 42,
            email: "kirill.salykin@gmail.com".to_string(),
            name: "kirill Salykin".to_string(),
        })
        .await
        .unwrap();

    // TODO: check if allowed to based on plan limit
    // TODO: scheduled number of email
    Json(1)
}

#[derive(Debug, Serialize, Deserialize, ToSchema, sqlx::Type, sqlx::FromRow)]
#[sqlx(transparent)]
pub struct CampaignId(Uuid);

impl CampaignId {
    pub fn new() -> Self {
        Self(Uuid::now_v7())
    }
}

#[derive(Debug, Serialize, Deserialize, ToSchema, sqlx::Type, sqlx::FromRow)]
#[sqlx(transparent)]
pub struct CampaignName(String);

#[derive(Debug, Serialize, Deserialize, ToSchema, sqlx::Type, sqlx::FromRow)]
struct Campaign {
    id: CampaignId,
    name: CampaignName,
    email_source: Option<Value>,
}

#[derive(Debug, Deserialize, ToSchema)]
struct CreateCampaignInput {
    name: CampaignName,
}

#[derive(Debug, Deserialize, ToSchema)]
struct UpdateCampaignInput {
    name: CampaignName,
    email_source: Option<Value>,
    email_content: Option<String>,
}

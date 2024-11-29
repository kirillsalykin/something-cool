use crate::api::auth::User;
use crate::app;

use axum::{
    extract::{Multipart, State},
    http::StatusCode,
    response::{IntoResponse, Json},
    Extension,
};
use csv::ReaderBuilder;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::io::{Seek, Write};
use tempfile::tempfile;
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

pub fn router() -> OpenApiRouter<app::State> {
    OpenApiRouter::new().routes(routes!(get_leads, upload))
}

#[utoipa::path(
  get,
  path = "",
  responses((status = OK, body = Vec<Lead>)))]
async fn get_leads(
    State(db): State<PgPool>,
    Extension(user): Extension<User>,
) -> impl IntoResponse {
    // TODO: handle errors here
    let leads = sqlx::query_as::<_, Lead>("select * from lead where user_id = $1")
        .bind(&user.id)
        .fetch_all(&db)
        .await
        .unwrap();

    Json(leads)
}

#[derive(ToSchema)]
struct LeadsUpload {
    #[schema(value_type = String, format = Binary)]
    #[allow(dead_code)]
    file: String,
}

#[utoipa::path(
    post,
    request_body(content = inline(LeadsUpload), content_type = "multipart/form-data"),
    path = "",
    responses(
        (status = 200)
    ),
    )]
async fn upload(
    State(db): State<PgPool>,
    Extension(user): Extension<User>,

    mut multipart: Multipart,
) -> impl IntoResponse {
    // TODO: handle errors
    if let Some(mut field) = multipart.next_field().await.unwrap() {
        if field.name().unwrap() == "file" {
            let mut file = tempfile().unwrap();

            while let Some(chunk) = field.chunk().await.expect("Failed to read chunk") {
                file.write_all(&chunk).unwrap();
            }

            file.rewind().unwrap();

            let reader = ReaderBuilder::new().has_headers(true).from_reader(file);

            let mut chunk = Chunk::new(10_000); // Initialize Chunk with chunk_size

            for result in reader.into_records() {
                if let Ok(record) = result {
                    if let Some(s) = record.get(0) {
                        chunk.push(Email::from(s));

                        if chunk.is_full() {
                            upsert_chunk(&db, &chunk, &user).await;
                            chunk.clear();
                        }
                    }
                }
            }

            if !chunk.is_empty() {
                upsert_chunk(&db, &chunk, &user).await;
            }
        }
    }

    StatusCode::NO_CONTENT
}

async fn upsert_chunk(db: &PgPool, chunk: &Chunk, user: &User) {
    // TODO: handle errors
    sqlx::query(
        "
          insert into lead(email, user_id)
          select unnest($1::text[]), $2
          on conflict (email, user_id) do nothing;
        ",
    )
    .bind(&chunk.emails)
    .bind(&user.id)
    .execute(db)
    .await
    .unwrap();
}

struct Chunk {
    emails: Vec<Email>,
    size: usize,
}

impl Chunk {
    fn new(size: usize) -> Self {
        Self {
            emails: Vec::with_capacity(size),
            size,
        }
    }

    fn clear(&mut self) {
        self.emails.clear();
    }

    fn push(&mut self, email: Email) {
        self.emails.push(email);
    }

    fn is_full(&self) -> bool {
        self.emails.len() == self.size
    }

    fn is_empty(&self) -> bool {
        self.emails.is_empty()
    }
}

#[derive(Debug, Serialize, Deserialize, ToSchema, sqlx::Type, sqlx::FromRow)]
struct Lead {
    email: Email,
}

#[derive(Debug, Serialize, Deserialize, ToSchema, sqlx::Type, sqlx::FromRow)]
#[sqlx(transparent)]
struct Email(String);

impl Email {
    fn from(s: &str) -> Self {
        Self(s.trim().to_string())
    }
}

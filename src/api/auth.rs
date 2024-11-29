use axum::{
    extract::{Request, State},
    http::{header, StatusCode},
    middleware::Next,
    response::Response,
};
use cached::proc_macro::cached;
use jsonwebtoken::jwk::{Jwk, JwkSet};
use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};
use reqwest;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use crate::configuration::CognitoConfig;

pub async fn authorization(
    State(db): State<PgPool>,
    State(cognito): State<CognitoConfig>,
    mut req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let token = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|header| header.to_str().ok())
        .and_then(|header| header.strip_prefix("Bearer "));

    if let Some(token) = token {
        if let Some(user) = authorize(cognito, db, token).await {
            req.extensions_mut().insert(user);
            return Ok(next.run(req).await);
        }
    }

    Err(StatusCode::UNAUTHORIZED)
}

async fn authorize(cognito: CognitoConfig, db: PgPool, token: &str) -> Option<User> {
    let user_id = verify_token(cognito, &token).await?;

    let user = sqlx::query_as::<_, User>("select * from \"user\" where id = $1")
        .bind(&user_id)
        .fetch_optional(&db)
        .await
        .unwrap();

    if user.is_some() {
        user
    } else {
        sqlx::query("insert into \"user\" (id) values ($1)")
            .bind(&user_id)
            .execute(&db)
            .await
            .ok()?;
        Some(User { id: user_id })
    }
}

// Amazon Cognito might rotate signing keys in your user pool. As a best practice, cache public keys in your app, using the kid as a cache key, and refresh the cache periodically.
// Compare the kid in the tokens that your app receives to your cache.
// If you receive a token with the correct issuer but a different kid, Amazon Cognito might have rotated the signing key. Refresh the cache from your user pool jwks_uri endpoint.
async fn verify_token(cognito: CognitoConfig, token: &str) -> Option<UserId> {
    let header = jsonwebtoken::decode_header(token).ok()?;
    let kid = header.kid?;

    let mut validation = Validation::new(Algorithm::from(header.alg));
    validation.set_issuer(&[cognito.issuer()]);
    validation.set_required_spec_claims(&["exp", "iss", "sub"]);

    let jwk = fetch_jwk(cognito, kid).await?;
    let decoding_key = DecodingKey::from_jwk(&jwk).ok()?;

    let token_data = decode::<Claims>(token, &decoding_key, &validation).ok()?;
    Some(token_data.claims.sub)
}

#[cached(time = 86400, sync_writes = true)]
async fn fetch_jwk(cognito: CognitoConfig, kid: String) -> Option<Jwk> {
    let jwk_set = fetch_jwks(cognito.jwks_uri()).await;
    jwk_set.find(&kid).cloned()
}

async fn fetch_jwks(url: String) -> JwkSet {
    reqwest::get(url).await.unwrap().json().await.unwrap()
}

#[derive(Debug, Deserialize, Serialize)]
struct Claims {
    sub: UserId,
}

#[derive(Debug, Clone, Deserialize, Serialize, sqlx::FromRow)]
pub struct User {
    pub id: UserId,
}

#[derive(Debug, Clone, Deserialize, Serialize, sqlx::Type)]
#[sqlx(transparent)]
pub struct UserId(Uuid);

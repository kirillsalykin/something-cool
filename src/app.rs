use crate::api::auth;
use crate::api::campaigns;
use crate::api::leads;
use crate::configuration::CognitoConfig;
use crate::configuration::Config;
use crate::database;

use axum::{extract::FromRef, middleware};
use lettre::message::header::ContentType;
use lettre::transport::smtp::authentication::Credentials;
use lettre::{Message, SmtpTransport, Transport};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::sync::Arc;
use std::time::Duration;
use tokio::{signal, task};
use tower_http::cors::CorsLayer;
use tower_http::timeout::TimeoutLayer;
use underway::{self, Job, To};
use utoipa::OpenApi;
use utoipa_axum::router::OpenApiRouter;
use utoipa_swagger_ui::SwaggerUi;

#[derive(Clone, Deserialize, Serialize)]
pub struct WelcomeEmail {
    pub user_id: i32,
    pub email: String,
    pub name: String,
}

pub type WelcomeEmailJob = Job<WelcomeEmail, ()>;

#[derive(Clone)]
pub struct State(Arc<InnerState>);

impl FromRef<State> for PgPool {
    fn from_ref(state: &State) -> Self {
        state.0.db.clone()
    }
}

impl FromRef<State> for CognitoConfig {
    fn from_ref(state: &State) -> Self {
        state.0.cognito.clone()
    }
}

impl FromRef<State> for WelcomeEmailJob {
    fn from_ref(state: &State) -> Self {
        state.0.job.clone()
    }
}

pub struct InnerState {
    db: PgPool,
    cognito: CognitoConfig,
    job: WelcomeEmailJob,
}

#[derive(OpenApi)]
struct ApiDoc;

pub struct App {}

impl App {
    pub async fn run(config: Config) -> () {
        let pool = database::get_connection_pool(config.database).await;
        let job = Job::builder()
            .step(
                |_ctx,
                 WelcomeEmail {
                     user_id,
                     email,
                     name,
                 }| async move {
                    let creds = Credentials::new("name".to_owned(), "pass".to_owned());

                    // Open a remote connection to gmail
                    let mailer = SmtpTransport::relay("mail.smtp2go.com")
                        .unwrap()
                        .credentials(creds)
                        .build();

                    let email = Message::builder()
                        .from("info@prostor.email".parse().unwrap())
                        .to(email.parse().unwrap())
                        .subject("IT_WORKS")
                        .header(ContentType::TEXT_HTML)
                        .body(String::from("BODY"))
                        .unwrap();

                    mailer.send(&email).unwrap();

                    To::done()
                },
            )
            .name("job-queue")
            .pool(pool.clone())
            .build()
            .await
            .unwrap();

        let state = State(Arc::new(InnerState {
            db: pool.clone(),
            cognito: config.cognito,
            job: job.clone(),
        }));

        let (router, api) = OpenApiRouter::with_openapi(ApiDoc::openapi())
            .nest("/campaigns", campaigns::router())
            .nest("/leads", leads::router())
            .split_for_parts();

        let router = router
            .route_layer(middleware::from_fn_with_state(
                state.clone(),
                auth::authorization,
            ))
            .with_state(state.clone())
            .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", api))
            .layer((
                CorsLayer::permissive(),
                TimeoutLayer::new(Duration::from_secs(10)),
            ));

        let listener = tokio::net::TcpListener::bind(config.app.addr)
            .await
            .unwrap();
        let addr = listener.local_addr().unwrap();
        println!("listening on {}", addr);

        tokio::join!(
            task::spawn({
                let pool = pool.clone();
                async move {
                    shutdown_signal().await;
                    underway::queue::graceful_shutdown(&pool).await.unwrap();
                }
            }),
            task::spawn(async move { job.run().await }),
            task::spawn(async move {
                axum::serve(listener, router)
                    .with_graceful_shutdown(shutdown_signal())
                    .await
            })
        );
    }
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}

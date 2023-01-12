use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;

use axum::extract::{Query, State};
use axum::response::{IntoResponse, Redirect};
use axum::routing::get;
use axum::Router;

use anyhow::{bail, Result};
use serde::Deserialize;
use sqlx::postgres::PgPoolOptions;
use sqlx::Pool;
use tracing::debug;
use tracing_subscriber::{
    prelude::__tracing_subscriber_SubscriberExt, util::SubscriberInitExt, EnvFilter, Registry,
};
use tracing_tree::HierarchicalLayer;

mod schema;

#[tracing::instrument(skip_all)]
async fn handle_oauth_redirect(
    Query(params): Query<HashMap<String, String>>,
    State(pool): State<Arc<Pool<sqlx::Postgres>>>,
) -> impl IntoResponse {
    let (Some(code), Some(state), Some(scope)) = (params.get("code"), params.get("state"), params.get("scope")) else {
        return "missing google oath params";
    };

    if !scope.contains("https://www.googleapis.com/auth/drive") {
        return "please provide google drive permission";
    }

    let mut client = get_oauth_client("", "").await.unwrap();
    let access_token = client.get_access_token(code, state).await;

    if let Ok(access_token) = access_token {
        if access_token.access_token.is_empty() {
            return "old oauth request, please re-onboard";
        }
        debug!("{:#?}", access_token);
        #[derive(Deserialize)]
        struct User {
            email: String,
        }
        if let Ok(resp) = reqwest::Client::new()
            .get("https://www.googleapis.com/oauth2/v2/userinfo")
            .query(&[("oauth_token", &access_token.access_token)])
            .send()
            .await
        {
            let now = sqlx::types::time::OffsetDateTime::now_utc().unix_timestamp();
            let token_expiration = sqlx::types::time::OffsetDateTime::from_unix_timestamp(
                now + access_token.expires_in,
            )
            .unwrap();
            let refresh_expiration = sqlx::types::time::OffsetDateTime::from_unix_timestamp(
                now + access_token.refresh_token_expires_in,
            )
            .unwrap();
            if let Ok(body) = resp.json::<User>().await {
                if !body.email.is_empty() {
                    struct UserId {
                        id: i32,
                    }

                    let mut userid = None;
                    if let Ok(Some(user_id)) = sqlx::query_as!(
                        UserId,
                        "SELECT id from users where email = $1 ",
                        &body.email
                    )
                    .fetch_optional(pool.as_ref())
                    .await
                    {
                        userid = Some(user_id.id);
                    } else if sqlx::query!("INSERT INTO users(email) values($1)", &body.email)
                        .execute(pool.as_ref())
                        .await
                        .is_ok()
                    {
                        if let Ok(user_id) = sqlx::query_as!(
                            UserId,
                            "SELECT id from users where email = $1 ",
                            &body.email
                        )
                        .fetch_one(pool.as_ref())
                        .await
                        {
                            userid = Some(user_id.id);
                        }
                    }
                    let Some(userid) = userid else {
                        return "database failure";
                    };
                    if sqlx::query!("DELETE FROM auth_tokens where user_id=$1", userid)
                            .execute(pool.as_ref())
                            .await
                            .is_ok() && sqlx::query!("INSERT INTO auth_tokens(user_id, token, token_expiration, refresh_token, refresh_token_expiration) values ($1, $2, $3, $4, $5)", userid, access_token.access_token, token_expiration, access_token.refresh_token, refresh_expiration).execute(pool.as_ref()).await.is_ok() {
                            return "success!";
                        }
                }
            }
        }
    } else {
        return "failed to get access token";
    }

    "success!"
}

/// Build google_drive::Client from environment variables
#[tracing::instrument(skip_all)]
async fn get_oauth_client(token: &str, refresh_token: &str) -> Result<google_drive::Client> {
    let (Ok(client_id), Ok(client_secret), Ok(redirect_uri)) = (std::env::var("GOOGLE_DRIVE_CLIENT_ID"), std::env::var("GOOGLE_DRIVE_CLIENT_SECRET"), std::env::var("GOOGLE_DRIVE_REDIRECT_URI")) else {
        bail!("missing needed environment variables");
    };

    Ok(google_drive::Client::new(
        client_id,
        client_secret,
        redirect_uri,
        token,
        refresh_token,
    ))
}

#[tokio::main]
async fn main() -> Result<()> {
    Registry::default()
        .with(EnvFilter::from_default_env())
        .with(
            HierarchicalLayer::new(2)
                .with_targets(true)
                .with_bracketed_fields(true),
        )
        .init();

    dotenvy::dotenv().ok();

    let google_oauth_uri = get_oauth_client("", "").await?.user_consent_url(&[
        String::from("https://www.googleapis.com/auth/drive"),
        String::from("https://www.googleapis.com/auth/userinfo.email"),
    ]);

    let pool = Arc::new(
        PgPoolOptions::new()
            .max_connections(5)
            .connect(&std::env::var("DATABASE_URL")?)
            .await?,
    );

    let app = Router::new()
        .route(
            "/onboard",
            get(|| async move { Redirect::to(&google_oauth_uri) }),
        )
        .route("/authorized", get(handle_oauth_redirect))
        .with_state(pool);

    let addr = SocketAddr::from(([0, 0, 0, 0], 1234));
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();

    Ok(())
}

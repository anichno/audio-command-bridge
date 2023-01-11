use std::collections::HashMap;
use std::net::SocketAddr;

use axum::extract::Query;
use axum::response::{IntoResponse, Redirect};
use axum::routing::get;
use axum::Router;

use anyhow::{bail, Result};
use tracing::debug;
use tracing_subscriber::{
    prelude::__tracing_subscriber_SubscriberExt, util::SubscriberInitExt, EnvFilter, Registry,
};
use tracing_tree::HierarchicalLayer;

#[tracing::instrument(skip_all)]
async fn handle_oauth_redirect(Query(params): Query<HashMap<String, String>>) -> impl IntoResponse {
    let (Some(code), Some(state), Some(scope)) = (params.get("code"), params.get("state"), params.get("scope")) else {
        return "missing google oath params";
    };

    if !scope.contains("https://www.googleapis.com/auth/drive") {
        return "please provide google drive permission";
    }

    let mut client = get_oauth_client("", "").await.unwrap();
    let access_token = client.get_access_token(code, state).await;

    if let Ok(access_token) = access_token {
        debug!("{:#?}", access_token);
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

    dotenv::dotenv().ok();

    let google_oauth_uri = get_oauth_client("", "").await?.user_consent_url(&[
        String::from("https://www.googleapis.com/auth/drive"),
        String::from("https://www.googleapis.com/auth/userinfo.email"),
    ]);

    let app = Router::new()
        .route(
            "/onboard",
            get(|| async move { Redirect::to(&google_oauth_uri) }),
        )
        .route("/authorized", get(handle_oauth_redirect));

    let addr = SocketAddr::from(([0, 0, 0, 0], 1234));
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();

    Ok(())
}

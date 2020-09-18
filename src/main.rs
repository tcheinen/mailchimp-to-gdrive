#[macro_use]
extern crate log;

#[macro_use]
extern crate serde_json;

mod structures;

use log::Level;
use serde_json::Value;
use std::borrow::BorrowMut;
use std::collections::HashMap;
use warp::{http::Response, Filter, Rejection};
use yup_oauth2::authenticator::Authenticator;
use yup_oauth2::{
    AccessToken, InstalledFlowAuthenticator, InstalledFlowReturnMethod, ServiceAccountAuthenticator,
};

use std::error::Error;
use structures::*;

static DRIVE_ID: &str = "0ADjTANDsCZhEUk9PVA";

async fn get_token() -> Result<AccessToken, Box<dyn std::error::Error>> {
    //TODO make this caching based on ttl
    let secret = yup_oauth2::read_service_account_key("clientsecret.json").await?;

    let auth = ServiceAccountAuthenticator::builder(secret).build().await?;

    Ok(auth
        .token(&vec!["https://www.googleapis.com/auth/drive"])
        .await?)
}

async fn subscribe(email: String, token: AccessToken) -> Result<(), Box<dyn std::error::Error>> {
    let url = format!(
        "https://www.googleapis.com/drive/v3/files/{}/permissions",
        DRIVE_ID
    );
    let client = reqwest::Client::new();

    client
        .post(&url)
        .bearer_auth(token.as_str())
        .query(&[("supportsAllDrives", "true"), ("alt", "json")])
        .json(
            &[
                ("role".to_string(), "reader".to_string()),
                ("type".to_string(), "user".to_string()),
                ("emailAddress".to_string(), email),
            ]
            .iter()
            .map(|x| x.clone())
            .collect::<HashMap<String, String>>(),
        )
        .send()
        .await?
        .error_for_status()
        .map(|_| ())?;

    Ok(())
}

async fn unsubscribe(email: String, token: AccessToken) -> Result<(), Box<dyn std::error::Error>> {
    let permission_id: String = {
        // API v3 has no method to translate emails to permission ids??
        let url = format!(
            "https://www.googleapis.com/drive/v2/permissionIds/{}",
            email
        );
        let client = reqwest::Client::new();
        serde_json::from_str::<GetIdForEmailResponse>(
            &client
                .get(&url)
                .bearer_auth(token.as_str())
                .send()
                .await?
                .text()
                .await?,
        )?
        .id
    };

    let url = format!(
        "https://www.googleapis.com/drive/v3/files/{}/permissions/{}",
        DRIVE_ID, permission_id
    );
    let client = reqwest::Client::new();

    client
        .delete(&url)
        .bearer_auth(token.as_str())
        .query(&[("supportsAllDrives", "true")])
        .send()
        .await?
        .error_for_status()
        .map(|_| ())?;

    Ok(())
}

async fn handle_mailchimp_hook(body: HookBody) -> Result<impl warp::Reply, warp::Rejection> {
    let token = get_token().await.unwrap();

    match body.action {
        HookAction::Subscribe => {
            info!("Adding permission for {}", body.email);
            match subscribe(body.email.clone(), token).await {
                Ok(_) => Ok(warp::reply()),
                Err(_) => Err(warp::reject()),
            }
        }
        HookAction::Unsubscribe => {
            info!("Removing permission for {}", body.email);
            match unsubscribe(body.email.clone(), token).await {
                Ok(_) => Ok(warp::reply()),
                Err(_) => Err(warp::reject()),
            }
        }
        HookAction::Other => {
            info!("Received hook with unsupported action type, this likely means that the webhook is configured to send more events than this program needs.  \n {:?}", body);
            Err(warp::reject())
        }
    }
}

#[tokio::main]
async fn main() {
    env_logger::init();

    let webhook = warp::post()
        .and(warp::path("mailchimp"))
        .and(warp::body::json())
        .and_then(handle_mailchimp_hook);
    warp::serve(webhook).run(([127, 0, 0, 1], 3030)).await;
}

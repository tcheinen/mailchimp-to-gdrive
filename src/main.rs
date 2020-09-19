#[macro_use]
extern crate log;

mod options;
mod structures;

use std::collections::HashMap;
use warp::{Filter, Rejection, Reply};
use yup_oauth2::{AccessToken, ServiceAccountAuthenticator};

use once_cell::sync::OnceCell;
use options::*;
use std::convert::Infallible;
use structopt::StructOpt;
use structures::*;
use warp::http::StatusCode;

static ARGUMENTS: OnceCell<Arguments> = OnceCell::new();

async fn handle_rejection(err: Rejection) -> Result<impl Reply, Infallible> {
    let code;
    let message;

    if err.is_not_found() {
        code = StatusCode::NOT_FOUND;
        message = "NOT_FOUND";
    } else if let Some(e) = err.find::<StringRejection>() {
        code = StatusCode::BAD_REQUEST;
        message = &e.message;
    } else {
        eprintln!("unhandled rejection: {:?}", err);
        code = StatusCode::INTERNAL_SERVER_ERROR;
        message = "UNHANDLED_REJECTION";
    }

    let json = warp::reply::json(&ErrorMessage {
        code: code.as_u16(),
        message: message.into(),
    });

    Ok(warp::reply::with_status(json, code))
}

async fn get_token() -> Result<AccessToken, Box<dyn std::error::Error>> {
    //TODO make this caching based on ttl
    let secret = yup_oauth2::read_service_account_key("clientsecret.json").await?;

    let auth = ServiceAccountAuthenticator::builder(secret).build().await?;

    Ok(auth
        .token(&["https://www.googleapis.com/auth/drive"])
        .await?)
}

async fn subscribe(email: String, token: AccessToken) -> Result<(), Box<dyn std::error::Error>> {
    let url = format!(
        "https://www.googleapis.com/drive/v3/files/{}/permissions",
        ARGUMENTS.get().expect("Arguments to be defined").drive_id,
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
            .cloned()
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
        ARGUMENTS.get().expect("Arguments to be defined").drive_id,
        permission_id
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
                Err(e) => Err(warp::reject::custom(StringRejection::new(&e.to_string()))),
            }
        }
        HookAction::Unsubscribe => {
            info!("Removing permission for {}", body.email);
            match unsubscribe(body.email.clone(), token).await {
                Ok(_) => Ok(warp::reply()),
                Err(e) => Err(warp::reject::custom(StringRejection::new(&e.to_string()))),
            }
        }
        HookAction::Other => {
            info!("Received hook with unsupported action type, this likely means that the webhook is configured to send more events than this program needs.  \n {:?}", body);
            Err(warp::reject::custom(StringRejection::new(
                "Unsupported action type",
            )))
        }
    }
}

#[tokio::main]
async fn main() {
    env_logger::init();

    ARGUMENTS
        .set(Arguments::from_args())
        .expect("Arguments to parse");

    let webhook = warp::post()
        .and(warp::path("mailchimp"))
        .and(warp::body::json())
        .and_then(handle_mailchimp_hook)
        .recover(handle_rejection);
    warp::serve(webhook)
        .run((
            [127, 0, 0, 1],
            ARGUMENTS.get().expect("Arguments to be defined").port,
        ))
        .await;
}

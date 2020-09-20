#[macro_use]
extern crate log;

mod drive;
mod error;
mod options;
mod structures;

use std::collections::HashMap;
use warp::{Filter, Rejection, Reply};
use yup_oauth2::{AccessToken, ServiceAccountAuthenticator, ServiceAccountKey};

use once_cell::sync::OnceCell;
use options::*;
use std::convert::Infallible;
use structopt::StructOpt;
use structures::*;
use warp::http::StatusCode;

use drive::get_token;

static ARGUMENTS: OnceCell<Arguments> = OnceCell::new();
static SERVICE_ACCOUNT_KEY: OnceCell<ServiceAccountKey> = OnceCell::new();

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

async fn handle_mailchimp_hook(body: HookBody) -> Result<impl warp::Reply, warp::Rejection> {
    match body.action {
        HookAction::Subscribe => {
            info!("Adding permission for {}", body.email);
            match drive::add_user(body.email.clone()).await {
                Ok(_) => Ok(warp::reply()),
                Err(e) => Err(warp::reject::custom(StringRejection::new(&e.to_string()))),
            }
        }
        HookAction::Unsubscribe => {
            info!("Removing permission for {}", body.email);
            match drive::remove_user(body.email.clone()).await {
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
    SERVICE_ACCOUNT_KEY.set(
        yup_oauth2::read_service_account_key("clientsecret.json")
            .await
            .expect("clientsecret.json to exist"),
    );

    info!(
        "Starting up with bot account {}",
        SERVICE_ACCOUNT_KEY
            .get()
            .expect("SERVICE_ACCOUNT_KEY to be defined")
            .client_email
            .as_str()
    );

    ARGUMENTS
        .set(Arguments::from_args())
        .expect("Arguments to parse");

    info!(
        "Changing permissions for the following IDs: {:?}",
        ARGUMENTS
            .get()
            .expect("Arguments to be defined")
            .drive_id
            .as_slice()
    );

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

use crate::error::ErrorKind;
use crate::structures::GetIdForEmailResponse;
use crate::{error, ARGUMENTS};
use serde_json::Value;
use std::collections::HashMap;
use yup_oauth2::{AccessToken, ServiceAccountAuthenticator};

/// API calls for Google Drive

pub async fn get_token() -> Result<AccessToken, error::ErrorKind> {
    //TODO make this caching based on ttl

    let auth = ServiceAccountAuthenticator::builder(
        crate::SERVICE_ACCOUNT_KEY
            .get()
            .expect("SERVICE_ACCOUNT_KEY to be defined")
            .clone(),
    )
    .build()
    .await?;

    Ok(auth
        .token(&["https://www.googleapis.com/auth/drive"])
        .await?)
}

pub async fn add_user(email: String) -> Result<(), Box<dyn std::error::Error>> {
    let token = get_token().await.unwrap();
    for drive_id in ARGUMENTS
        .get()
        .expect("Arguments to be defined")
        .drive_id
        .iter()
    {
        let url = format!(
            "https://www.googleapis.com/drive/v3/files/{}/permissions",
            drive_id,
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
                    ("emailAddress".to_string(), email.clone()),
                ]
                .iter()
                .cloned()
                .collect::<HashMap<String, String>>(),
            )
            .send()
            .await?
            .error_for_status()
            .map(|_| {
                ErrorKind::ServerError(format!("Attempted to add user permission ({})", &email))
            })?;
    }
    Ok(())
}

pub async fn remove_user(email: String) -> Result<(), Box<dyn std::error::Error>> {
    let token = get_token().await.unwrap();

    let permission_id: String = {
        // API v3 has no method to translate emails to permission ids??
        let url = format!(
            "https://www.googleapis.com/drive/v2/permissionIds/{}",
            email
        );
        let client = reqwest::Client::new();
        let val: Value = serde_json::from_str(
            &client
                .get(&url)
                .bearer_auth(token.as_str())
                .send()
                .await?
                .text()
                .await?,
        )?;
        if let Value::Object(map) = val {
            let id_val = map
                .get("id")
                .ok_or(format!("Response from server missing id field"))?;
            if let Value::String(id) = id_val {
                Ok(id.clone())
            } else {
                Err(ErrorKind::IDRetrievalError(format!(
                    "Response from server has ID field but it is not a string ({:?})",
                    id_val
                )))
            }
        } else {
            Err(ErrorKind::IDRetrievalError(String::from(
                "Response from server not valid JSON",
            )))
        }
    }?;

    for drive_id in ARGUMENTS
        .get()
        .expect("Arguments to be defined")
        .drive_id
        .iter()
    {
        let url = format!(
            "https://www.googleapis.com/drive/v3/files/{}/permissions/{}",
            drive_id, permission_id
        );
        let client = reqwest::Client::new();

        client
            .delete(&url)
            .bearer_auth(token.as_str())
            .query(&[("supportsAllDrives", "true")])
            .send()
            .await?
            .error_for_status()
            .map(|_| {
                ErrorKind::ServerError(format!("Attempted to remove user permission ({})", email))
            })?;
    }

    Ok(())
}

use std::sync::Arc;

use auth_token::AuthToken;
use crates_payload::CratesPayload;
use poem::{
    handler,
    http::StatusCode,
    web::{Data, Json, Path},
    IntoResponse, Response,
};
use serde::{Deserialize, Serialize};

use crate::storage::Storage;

mod auth_token;
mod crates_payload;

pub struct HttpContext {
    pub authorization: Option<String>,
    pub endpoint: String,
    pub storage: Storage,
}

#[derive(Serialize)]
struct CreateNewReponseWarn {
    invalid_categories: Vec<String>,
    invalid_badges: Vec<String>,
    other: Vec<String>,
}

#[derive(Serialize)]
struct CreateNewResponse {
    warnings: CreateNewReponseWarn,
}

#[derive(Deserialize)]
struct CrateMeta {
    name: String,
    vers: String,
}

#[derive(Serialize)]
struct GetConfigRes {
    dl: String,
    api: String,
    #[serde(rename = "auth-required")]
    auth_required: bool,
}

#[handler]
pub async fn get_config(Data(data): Data<&Arc<HttpContext>>) -> poem::Result<Json<GetConfigRes>> {
    Ok(Json(GetConfigRes {
        dl: format!("{}/index", data.endpoint),
        api: data.endpoint.clone(),
        auth_required: data.authorization.is_some(),
    }))
}

#[handler]
pub async fn get_pkg(
    Data(data): Data<&Arc<HttpContext>>,
    token: AuthToken,
    Path((_be, _md, pkg)): Path<(String, String, String)>,
) -> impl IntoResponse {
    if data.authorization.is_some() && !token.0.eq(&data.authorization) {
        return Response::builder()
            .status(StatusCode::UNAUTHORIZED)
            .header("Content-Type", "text/plain")
            .body("No permissioned");
    }
    log::info!("get_pkg {pkg}");
    match data.storage.get_crate(&pkg).await {
        Ok(content) => Response::builder()
            .status(StatusCode::OK)
            .header("Content-Type", "application/json")
            .body(content),
        Err(e) => Response::builder()
            .status(StatusCode::NOT_FOUND)
            .header("Content-Type", "text/plain")
            .body(e.to_string()),
    }
}

#[handler]
pub async fn down_pkg(
    Data(data): Data<&Arc<HttpContext>>,
    token: AuthToken,
    Path((pkg, ver)): Path<(String, String)>,
) -> poem::Result<Vec<u8>> {
    if data.authorization.is_some() && !token.0.eq(&data.authorization) {
        return Err(poem::Error::from_string(
            "No permissioned".to_string(),
            StatusCode::UNAUTHORIZED,
        ));
    }

    log::info!("down_pkg {pkg}/{ver}");
    let value = data
        .storage
        .down_crate(&pkg, &ver)
        .await
        .map_err(|e| poem::Error::from_string(e.to_string(), StatusCode::NOT_FOUND))?;
    Ok(value)
}

#[handler]
pub async fn create_pkg(
    Data(data): Data<&Arc<HttpContext>>,
    token: AuthToken,
    payload: CratesPayload,
) -> poem::Result<Json<CreateNewResponse>> {
    if data.authorization.is_some() && !token.0.eq(&data.authorization) {
        return Err(poem::Error::from_string(
            "No permissioned".to_string(),
            StatusCode::UNAUTHORIZED,
        ));
    }

    let meta: CrateMeta = serde_json::from_slice(&payload.meta_buf).expect("parse meta");
    match data
        .storage
        .save_crate(&meta.name, &meta.vers, payload.meta_buf, payload.crate_buf)
        .await
    {
        Ok(_) => Ok(Json(CreateNewResponse {
            warnings: CreateNewReponseWarn {
                invalid_categories: vec![],
                invalid_badges: vec![],
                other: vec![],
            },
        })),
        Err(err) => Err(poem::Error::from_string(
            err.to_string(),
            StatusCode::BAD_REQUEST,
        )),
    }
}

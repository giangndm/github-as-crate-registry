use crates_new_payload::CratesPayload;
use poem::{
    error::NotFound,
    get, handler,
    listener::TcpListener,
    middleware::Tracing,
    put,
    web::{Json, Path},
    EndpointExt, Route, Server,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

mod crates_new_payload;

#[tokio::main]
async fn main() {
    if std::env::var_os("RUST_LOG").is_none() {
        std::env::set_var("RUST_LOG", "info");
    }
    tracing_subscriber::fmt::init();

    let app = Route::new()
        .at("/api/v1/crates/new", put(create_pkg))
        .at("/index/config.json", get(get_config))
        .at("/index/:pkg/:ver/download", get(down_pkg))
        .at("/index/:p1/:p2/:p3", get(get_pkg))
        .with(Tracing);

    Server::new(TcpListener::bind("0.0.0.0:3000"))
        .run(app)
        .await
        .expect("Should run");
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
    auth_required: bool,
}

#[handler]
async fn get_config() -> poem::Result<Json<GetConfigRes>> {
    Ok(Json(GetConfigRes {
        dl: "http://localhost:3000/index".to_string(),
        api: "http://localhost:3000".to_string(),
        auth_required: false,
    }))
}

#[handler]
async fn get_pkg(Path((be, md, pkg)): Path<(String, String, String)>) -> poem::Result<Json<Value>> {
    log::info!("get_pkg {pkg}");
    let res = std::fs::read(format!("/tmp/crates/{pkg}.json")).map_err(|e| NotFound(e))?;
    let value = serde_json::from_slice::<Value>(&res).map_err(|e| NotFound(e))?;
    Ok(Json(value))
}

#[handler]
async fn down_pkg(Path((pkg, ver)): Path<(String, String)>) -> poem::Result<Vec<u8>> {
    log::info!("down_pkg {pkg}/{ver}");
    // TODO: work with version
    let res = std::fs::read(format!("/tmp/crates/{pkg}.crate")).map_err(|e| NotFound(e))?;
    Ok(res)
}

#[handler]
async fn create_pkg(payload: CratesPayload) -> poem::Result<Json<CreateNewResponse>> {
    let meta: CrateMeta = serde_json::from_slice(&payload.meta_buf).expect("parse meta");

    let mut json =
        serde_json::from_slice::<Value>(&payload.meta_buf).expect("Should convert meta to json");
    if json.get("cksum").is_none() {
        json["cksum"] = sha256::digest(&payload.crate_buf).into();
    }
    if let Some(deps) = json.get_mut("deps") {
        if let Some(deps) = deps.as_array_mut() {
            for dep in deps {
                dep["req"] = dep["version_req"].clone();
            }
        }
    }
    let meta_new_buf = json.to_string();

    std::fs::write(format!("/tmp/crates/{}.json", meta.name), &meta_new_buf)
        .expect("should save to file");
    std::fs::write(
        format!("/tmp/crates/{}.crate", meta.name),
        &payload.crate_buf,
    )
    .expect("should save to file");

    Ok(Json(CreateNewResponse {
        warnings: CreateNewReponseWarn {
            invalid_categories: vec![],
            invalid_badges: vec![],
            other: vec![],
        },
    }))
}

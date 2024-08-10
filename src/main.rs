use clap::Parser;
use futures_util::StreamExt;
use http_body_util::BodyExt;
use octocrab::params::repos::Reference;
use std::sync::Arc;

use crates_new_payload::CratesPayload;
use poem::{
    get, handler,
    http::StatusCode,
    listener::TcpListener,
    put,
    web::{Data, Json, Path},
    EndpointExt, IntoResponse, Response, Route, Server,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

mod crates_new_payload;

struct Context {
    owner: String,
    repo: String,
    instance: octocrab::Octocrab,
}

/// Simple program to create private crate registry with github repo as a storage
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Owner of repo
    #[arg(short, env, long)]
    owner: String,

    /// Repo name
    #[arg(short, env, long)]
    repo: String,

    /// Github Token for access repo
    #[arg(short, env, long)]
    github_token: String,
}

#[tokio::main]
async fn main() {
    if std::env::var_os("RUST_LOG").is_none() {
        std::env::set_var("RUST_LOG", "info");
    }
    tracing_subscriber::fmt::init();
    let args = Args::parse();

    let ctx = Context {
        owner: args.owner,
        repo: args.repo,
        instance: octocrab::OctocrabBuilder::new()
            .personal_token(args.github_token)
            .build()
            .expect("Build instance"),
    };

    let app = Route::new()
        .at("/api/v1/crates/new", put(create_pkg))
        .at("/index/config.json", get(get_config))
        .at("/index/:pkg/:ver/download", get(down_pkg))
        .at("/index/:p1/:p2/:p3", get(get_pkg))
        .data(Arc::new(ctx));
    // .with(Tracing);

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
async fn get_pkg(
    Data(data): Data<&Arc<Context>>,
    Path((_be, _md, pkg)): Path<(String, String, String)>,
) -> impl IntoResponse {
    log::info!("get_pkg {pkg}");
    match data.get_binary(&format!("meta/{pkg}.json")).await {
        Ok(content) => Response::builder()
            .status(StatusCode::OK)
            .header("Content-Type", "application/json")
            .body(content),
        Err(e) => Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .header("Content-Type", "text/plain")
            .body(e),
    }
}

#[handler]
async fn down_pkg(
    Data(data): Data<&Arc<Context>>,
    Path((pkg, ver)): Path<(String, String)>,
) -> poem::Result<Vec<u8>> {
    log::info!("down_pkg {pkg}/{ver}");
    let value = data
        .get_binary(&format!("pkgs/{pkg}/{ver}.crate"))
        .await
        .map_err(|e| poem::Error::from_string(e, StatusCode::NOT_FOUND))?;
    Ok(value)
}

#[handler]
async fn create_pkg(
    Data(data): Data<&Arc<Context>>,
    payload: CratesPayload,
) -> poem::Result<Json<CreateNewResponse>> {
    let meta: CrateMeta = serde_json::from_slice(&payload.meta_buf).expect("parse meta");
    let meta_path = format!("meta/{}.json", meta.name);
    let crate_path = format!("pkgs/{}/{}.crate", meta.name, meta.vers);

    log::info!("prepare for create new pkg {meta_path}, {crate_path}");

    if data.get_binary(&crate_path).await.is_ok() {
        log::error!("already existed");
        return Err(poem::Error::from_string(
            "Already existed",
            StatusCode::BAD_REQUEST,
        ));
    }

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
    let mut meta_new_buf = json.to_string().as_bytes().to_vec();

    if let Ok(mut old_meta) = data.get_binary(&meta_path).await {
        let sha = data.get_sha(&meta_path).await.map_err(|e| {
            poem::Error::from_string(e.to_string(), StatusCode::INTERNAL_SERVER_ERROR)
        })?;
        old_meta.push('\n' as u8);
        old_meta.append(&mut meta_new_buf);
        data.instance
            .repos(&data.owner, &data.repo)
            .update_file(
                &meta_path,
                "Add more version to old crate meta",
                &old_meta,
                sha,
            )
            .send()
            .await
            .map_err(|e| {
                poem::Error::from_string(e.to_string(), StatusCode::INTERNAL_SERVER_ERROR)
            })?;
    } else {
        data.instance
            .repos(&data.owner, &data.repo)
            .create_file(&meta_path, "Add new crate meta", &meta_new_buf)
            .send()
            .await
            .map_err(|e| {
                poem::Error::from_string(e.to_string(), StatusCode::INTERNAL_SERVER_ERROR)
            })?;
    };

    data.instance
        .repos(&data.owner, &data.repo)
        .create_file(&crate_path, "Add crate version", &payload.crate_buf)
        .send()
        .await
        .map_err(|e| poem::Error::from_string(e.to_string(), StatusCode::INTERNAL_SERVER_ERROR))?;

    Ok(Json(CreateNewResponse {
        warnings: CreateNewReponseWarn {
            invalid_categories: vec![],
            invalid_badges: vec![],
            other: vec![],
        },
    }))
}

impl Context {
    async fn get_sha(&self, path: &str) -> Result<String, String> {
        let res = self
            .instance
            .repos(&self.owner, &self.repo)
            .get_content()
            .path(path)
            .r#ref("main")
            .send()
            .await
            .map_err(|e| e.to_string())?;
        res.items
            .last()
            .ok_or("NotFound".to_string())
            .map(|i| i.sha.clone())
    }

    async fn get_binary(&self, path: &str) -> Result<Vec<u8>, String> {
        let res = self
            .instance
            .repos(&self.owner, &self.repo)
            .raw_file(Reference::Branch("main".to_string()), path)
            .await
            .map_err(|e| e.to_string())?;

        if res.status() != 200 {
            return Err("NotFound".to_string());
        }

        let mut buf = Vec::new();
        let mut body = res.into_body().into_data_stream();
        while let Some(Ok(chunk)) = body.next().await {
            log::info!("{path}: chunk {}", chunk.len());
            buf.append(&mut chunk.to_vec());
        }

        Ok(buf)
    }
}

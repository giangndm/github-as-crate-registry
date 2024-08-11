use clap::Parser;
use futures_util::StreamExt;
use http_body_util::BodyExt;
use octocrab::params::repos::Reference;
use poem::middleware::Tracing;
use std::sync::Arc;

use crates_new_payload::CratesPayload;
use poem::{
    get, handler,
    http::{header::AUTHORIZATION, StatusCode},
    listener::TcpListener,
    put,
    web::{Data, Json, Path},
    EndpointExt, FromRequest, IntoResponse, Response, Route, Server,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

mod crates_new_payload;

struct Context {
    owner: String,
    repo: String,
    branch: String,
    authorization: Option<String>,
    public_endpoint: String,
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

    /// Branch
    #[arg(short, env, long)]
    branch: String,

    /// Authorization fixed token
    #[arg(short, env, long)]
    authorization: Option<String>,

    /// Github Token for access repo
    #[arg(short, env, long)]
    github_token: String,

    /// Branch
    #[arg(short, env, long)]
    public_endpoint: String,
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
        branch: args.branch,
        authorization: args.authorization,
        public_endpoint: args.public_endpoint,
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
        .data(Arc::new(ctx))
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
    #[serde(rename = "auth-required")]
    auth_required: bool,
}

#[handler]
async fn get_config(Data(data): Data<&Arc<Context>>) -> poem::Result<Json<GetConfigRes>> {
    Ok(Json(GetConfigRes {
        dl: format!("{}/index", data.public_endpoint),
        api: data.public_endpoint.clone(),
        auth_required: data.authorization.is_some(),
    }))
}

#[handler]
async fn get_pkg(
    Data(data): Data<&Arc<Context>>,
    token: BearerToken,
    Path((_be, _md, pkg)): Path<(String, String, String)>,
) -> impl IntoResponse {
    if data.authorization.is_some() && !Some(token.0).eq(&data.authorization) {
        return Response::builder()
            .status(StatusCode::FORBIDDEN)
            .header("Content-Type", "text/plain")
            .body("No permissioned");
    }
    log::info!("get_pkg {pkg}");
    match data
        .get_binary(&format!("meta/{pkg}.json"), &data.branch)
        .await
    {
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
    token: BearerToken,
    Path((pkg, ver)): Path<(String, String)>,
) -> poem::Result<Vec<u8>> {
    if data.authorization.is_some() && !Some(token.0).eq(&data.authorization) {
        return Err(poem::Error::from_string(
            "No permissioned".to_string(),
            StatusCode::FORBIDDEN,
        ));
    }

    log::info!("down_pkg {pkg}/{ver}");
    let value = data
        .get_binary(&format!("pkgs/{pkg}/{ver}.crate"), &data.branch)
        .await
        .map_err(|e| poem::Error::from_string(e, StatusCode::NOT_FOUND))?;
    Ok(value)
}

#[handler]
async fn create_pkg(
    Data(data): Data<&Arc<Context>>,
    token: BearerToken,
    payload: CratesPayload,
) -> poem::Result<Json<CreateNewResponse>> {
    if data.authorization.is_some() && !Some(token.0).eq(&data.authorization) {
        return Err(poem::Error::from_string(
            "No permissioned".to_string(),
            StatusCode::FORBIDDEN,
        ));
    }

    let meta: CrateMeta = serde_json::from_slice(&payload.meta_buf).expect("parse meta");
    let meta_path = format!("meta/{}.json", meta.name);
    let crate_path = format!("pkgs/{}/{}.crate", meta.name, meta.vers);

    log::info!("prepare for create new pkg {meta_path}, {crate_path}");

    if data.get_binary(&crate_path, &data.branch).await.is_ok() {
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

    if let Ok(mut old_meta) = data.get_binary(&meta_path, &data.branch).await {
        let sha = data.get_sha(&meta_path, &data.branch).await.map_err(|e| {
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
    async fn get_sha(&self, path: &str, branch: &str) -> Result<String, String> {
        let res = self
            .instance
            .repos(&self.owner, &self.repo)
            .get_content()
            .path(path)
            .r#ref(branch)
            .send()
            .await
            .map_err(|e| e.to_string())?;
        res.items
            .last()
            .ok_or("NotFound".to_string())
            .map(|i| i.sha.clone())
    }

    async fn get_binary(&self, path: &str, branch: &str) -> Result<Vec<u8>, String> {
        let res = self
            .instance
            .repos(&self.owner, &self.repo)
            .raw_file(Reference::Branch(branch.to_string()), path)
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

struct BearerToken(pub String);

impl<'a> FromRequest<'a> for BearerToken {
    fn from_request(
        req: &'a poem::Request,
        _body: &mut poem::RequestBody,
    ) -> impl std::future::Future<Output = poem::Result<Self>> + Send {
        Self::from_request_without_body(req)
    }
    fn from_request_without_body(
        req: &'a poem::Request,
    ) -> impl std::future::Future<Output = poem::Result<Self>> + Send {
        async move {
            log::info!("[BearerToken] check headers {:?}", req.headers());
            if let Some(header) = req.header(AUTHORIZATION) {
                return Ok(BearerToken(header.to_string()));
            }
            Err(poem::Error::from_string(
                "Missing AUTHORIZATION",
                StatusCode::UNAUTHORIZED,
            ))
        }
    }
}

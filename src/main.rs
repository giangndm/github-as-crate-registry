use clap::Parser;
use http::{create_pkg, down_pkg, get_config, get_pkg, HttpContext};
use poem::{get, listener::TcpListener, middleware::Tracing, put, EndpointExt, Route, Server};
use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    sync::Arc,
};
use storage::Storage;

mod http;
mod storage;

/// Simple program to create private crate registry with github repo as a storage
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Http port
    #[arg(short, env, long, default_value_t = 3000)]
    port: u16,

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
    github_token: Option<String>,

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

    let storage = Storage::new(&args.owner, &args.repo, &args.branch, args.github_token);

    let app = Route::new()
        .at("/api/v1/crates/new", put(create_pkg))
        .at("/index/config.json", get(get_config))
        .at("/index/:pkg/:ver/download", get(down_pkg))
        .at("/index/:p1/:p2/:p3", get(get_pkg))
        .data(Arc::new(HttpContext {
            authorization: args.authorization,
            public_endpoint: args.public_endpoint,
            storage,
        }))
        .with(Tracing);

    Server::new(TcpListener::bind(SocketAddr::new(
        IpAddr::V4(Ipv4Addr::UNSPECIFIED),
        args.port,
    )))
    .run(app)
    .await
    .expect("Should run");
}

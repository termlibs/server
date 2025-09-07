extern crate core;

use axum::{
  extract::{Path, Query},
  http::StatusCode,
  response::{Html, IntoResponse},
  routing::get,
  Router,
};
use std::net::SocketAddr;
use tower_http::cors::CorsLayer;

mod app_downloader;
mod gh;
mod static_site;
mod supported_apps;
mod templates;
mod types;

use crate::app_downloader::{TargetDeployment, TargetOs};
use crate::gh::get_github_download_links;
use crate::supported_apps::{DownloadInfo, Repo, SupportedApp};
use crate::templates::TEMPLATES;
use crate::types::{ScriptResponse, StringList};
use log::{debug, info, warn};
use serde_json::json;
use std::env;
use std::path::PathBuf;
use std::sync::LazyLock;
use tera::Context;
use types::InstallQueryOptions;

const FAVICON: &[u8] = include_bytes!("../favicon.ico");
static TERMLIBS_ROOT: LazyLock<PathBuf> =
    LazyLock::new(|| PathBuf::from(env::var("TERMLIBS_ROOT").unwrap_or("../".into())));

fn setup_logger(log_level: &str) -> Result<(), fern::InitError> {
    let log_level = log_level.to_uppercase();
    let level = match log_level.as_str() {
        "DEBUG" => log::LevelFilter::Debug,
        "INFO" => log::LevelFilter::Info,
        "WARN" => log::LevelFilter::Warn,
        "ERROR" => log::LevelFilter::Error,
        _ => log::LevelFilter::Info,
    };
    fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "{} {}: {}",
                chrono::Local::now().format("[%Y-%m-%d %H:%M:%S]"),
                record.level(),
                message
            ));
        })
        .level(level)
        .chain(std::io::stdout())
        .apply()?;
    warn!("Logging initialized at level {}", log_level);
    Ok(())
}

async fn favicon() -> impl IntoResponse {
    (
        StatusCode::OK,
        [("Content-Type", "image/x-icon")],
        FAVICON
    )
}

async fn install_arbitrary_github_handler(
    Path((user, repo)): Path<(String, String)>,
    Query(mut q): Query<InstallQueryOptions>,
) -> impl IntoResponse {
    log::debug!("install_arbitrary_github_handler({:?}, {:?}) with {:#?}", user, repo, q);
    let unsupported_app = SupportedApp::new(
        "unsupported",
        Repo::github(&format!("{}/{}", user, repo)),
        "github",
    );

    let links = load_app(&mut q, &unsupported_app).await.unwrap();
    let json_links: Vec<serde_json::Value> = links.iter().map(|x| x.json()).collect();
    let tera_context = Context::from_value(
        json!({
            "app": "",
            "force": false,
            "quiet": false,
            "assets": json_links,
            "log_level": q.log_level.clone()
        })
    ).unwrap();
    let script = TEMPLATES.render("install.sh", &tera_context);

    ScriptResponse::new(format!("install.sh"), script.unwrap())
}

async fn install_handler(
    Path(app): Path<String>,
    Query(mut q): Query<InstallQueryOptions>,
) -> impl IntoResponse {
    log::debug!("install_handler({:?}, {:?})", app, q);
    q.set_app(app.clone());

    let supported_app = supported_apps::get_app(&app).unwrap();

    let links = load_app(&mut q, &supported_app).await.unwrap();
    let json_links: Vec<serde_json::Value> = links.iter().map(|x| x.json()).collect();
    let tera_context = Context::from_value(
        json!({
            "app": app,
            "force": false,
            "quiet": false,
            "assets": json_links,
            "log_level": q.log_level.clone()
        })
    ).unwrap();
    let script = TEMPLATES.render("install.sh", &tera_context);

    ScriptResponse::new(format!("install-{}.sh", app), script.unwrap())
}

async fn load_app(q: &mut InstallQueryOptions, supported_app: &SupportedApp) -> anyhow::Result<Vec<DownloadInfo>> {
    let arch = q.arch.clone();
    let os = q.os.clone();
    let version = q.version.clone();
    let target_deployment = TargetDeployment::new(os, arch);
    debug!("target_deployment loaded: {:#?}", target_deployment);
    get_github_download_links(&supported_app.repo, &target_deployment, &version).await
}

async fn root_handler() -> impl IntoResponse {
    info!("{:?}", "root");
    info!("{:?}", TERMLIBS_ROOT);
    let html = static_site::load_static("index.html").unwrap_or("".to_string());
    Html(html)
}

#[tokio::main]
async fn main() {
    // make sure the templates are loaded early to check for errors
    let _ = TEMPLATES.get_template_names().map(
        |name| { info!("template loaded: {}", name); }
    );

    let port = env::var("PORT")
        .unwrap_or("8080".to_string())
        .parse::<u16>()
        .unwrap();
    let log_level = env::var("LOG_LEVEL").unwrap_or("DEBUG".to_string());
    let listen_ip: String = env::var("LISTEN").unwrap_or("0.0.0.0".to_string());

    setup_logger(log_level.as_str()).unwrap_or(());

    info!("starting server at {:?}:{}", listen_ip, port);

    let app = Router::new()
        .route("/", get(root_handler))
        .route("/favicon.ico", get(favicon))
        .route("/install/{user}/{repo}", get(install_arbitrary_github_handler))
        .route("/install/{app}", get(install_handler))
        .layer(CorsLayer::permissive());

    let addr = format!("{}:{}", listen_ip, port).parse::<SocketAddr>().unwrap();
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();

    axum::serve(listener, app)
        .await
        .unwrap();
}

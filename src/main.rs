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
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;
mod domain;
mod error;
mod gh;
mod http;
mod static_site;
mod supported_apps;
mod templates;
mod services;

use crate::error::AppError;
use crate::domain::platform::{TargetArch, TargetOs};
use crate::services::installer;
use crate::templates::TEMPLATES;
use crate::http::query::{InstallMethod, InstallQueryOptions};
use crate::http::responses::ScriptResponse;
use log::{debug, info, warn};
use std::env;
use std::path::PathBuf;
use std::sync::LazyLock;

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
    (StatusCode::OK, [("Content-Type", "image/x-icon")], FAVICON)
}

#[utoipa::path(
  get,
  path = "/install/{user}/{repo}",
  params(
    ("user" = String, Path, description = "GitHub username"),
    ("repo" = String, Path, description = "GitHub repository name")
  ),
  responses(
    (status = 200, description = "Install script (bash)for arbitrary GitHub repository", body = ScriptResponse, content_type = "application/x-sh"),
    (status = 200, description = "Install script (powershell) for arbitrary GitHub repository", body = ScriptResponse, content_type = "application/x-powershell")
  ),
  tag = "install"
)]
async fn install_arbitrary_github_handler(
    Path((user, repo)): Path<(String, String)>,
    Query(mut q): Query<InstallQueryOptions>,
) -> Result<ScriptResponse, AppError> {
    debug!(
        "install_arbitrary_github_handler({:?}, {:?}) with {:#?}",
        user, repo, q
    );
    installer::build_arbitrary_github_install_script(&user, &repo, &mut q).await
}

#[utoipa::path(
  get,
  path = "/install/{app}",
  params(
    ("app" = String, Path, description = "Application name (e.g., yq, jq, gh)"),
    ("os" = Option<String>, Query, description = "target os"),
    ("arch" = Option<String>, Query, description = "target architecture", nullable),
    ("prefix" = Option<String>, Query, description = "install directory", nullable),
    ("version" = Option<String>, Query, description = "app version, default is latest", nullable)
  ),
  responses(
    (status = 200, description = "Install script (bash) for the application", body = ScriptResponse, content_type = "application/x-sh"),
    (status = 200, description = "Install script (powershell) for the application", body = ScriptResponse, content_type = "application/x-powershell")
  ),
  tag = "install"
)]
async fn install_handler(
    Path(app): Path<String>,
    Query(mut q): Query<InstallQueryOptions>,
) -> Result<ScriptResponse, AppError> {
    debug!("install_handler({:?}, {:?})", app, q);
    installer::build_supported_install_script(&app, &mut q).await
}

async fn root_handler() -> impl IntoResponse {
    info!("{:?}", "root");
    info!("{:?}", TERMLIBS_ROOT);
    let html = static_site::load_static("index.html").unwrap_or("".to_string());
    Html(html)
}

#[derive(OpenApi)]
#[openapi(
  info(
    title = "Termlibs API",
    version = "0.3.0",
    description = "Terminal library installer API - Generate install scripts for popular CLI tools",
    contact(
      name = "Termlibs",
      email = "adam@huganir.com",
      url = "https://github.com/termlibs"
    )
  ),
  servers(
    (url = "http://localhost:8000/v1", description = "Local server"),
    (url = "https://termlibs.dev/v1", description = "Production server")
  ),
  paths(
    install_handler,
    install_arbitrary_github_handler
  ),
  components(
    schemas(InstallQueryOptions, ScriptResponse, InstallMethod, TargetOs, TargetArch)
  ),
  tags(
    (name = "install", description = "Install script generation")
  )
)]
struct ApiDoc;

#[tokio::main]
async fn main() {
    // make sure the templates are loaded early to check for errors
    let _ = TEMPLATES.get_template_names().map(|name| {
        info!("template loaded: {}", name);
    });

    let port = env::var("PORT")
        .unwrap_or("8080".to_string())
        .parse::<u16>()
        .unwrap();
    let log_level = env::var("LOG_LEVEL").unwrap_or("DEBUG".to_string());
    let listen_ip: String = env::var("LISTEN").unwrap_or("0.0.0.0".to_string());

    setup_logger(log_level.as_str()).unwrap_or(());

    info!("starting server at {:?}:{}", listen_ip, port);

    let app = build_app();

    let addr = format!("{}:{}", listen_ip, port)
        .parse::<SocketAddr>()
        .unwrap();
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();

    axum::serve(listener, app).await.unwrap();
}

fn build_app() -> Router {
    let v1_router = Router::new()
        .route("/install/{user}/{repo}", get(install_arbitrary_github_handler))
        .route("/install/{app}", get(install_handler));

    Router::new()
        .route("/", get(root_handler))
        .route("/favicon.ico", get(favicon))
        .nest("/v1", v1_router)
        .merge(SwaggerUi::new("/swagger-ui").url("/openapi.json", ApiDoc::openapi()))
        .layer(CorsLayer::permissive())
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum_test::TestServer;

    async fn test_server() -> TestServer {
        let app = build_app();
        TestServer::new(app).unwrap()
    }

    #[tokio::test]
    async fn test_favicon() {
        let server = test_server().await;
        let response = server.get("/favicon.ico").await;
        response.assert_status_ok();
        response.assert_header("Content-Type", "image/x-icon");
    }

    #[tokio::test]
    async fn test_install_yutc() {
        let server = test_server().await;
        let response = server.get("/v1/install/yutc").await;
        response.assert_status_ok();
        response.assert_header("Content-Type", "application/x-sh");
    }
}

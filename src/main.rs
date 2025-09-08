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
mod app_downloader;
mod gh;
mod static_site;
mod supported_apps;
mod templates;
mod types;

use crate::app_downloader::TargetDeployment;
use crate::gh::get_github_download_links;
use crate::supported_apps::{DownloadInfo, Repo, SupportedApp};
use crate::templates::TEMPLATES;
use crate::types::{InstallMethod, InstallQueryOptions, ScriptResponse, TargetArch, TargetOs};
use log::{debug, info, warn};
use serde_json::json;
use std::env;
use std::path::PathBuf;
use std::sync::LazyLock;
use tera::Context;

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
    (status = 200, description = "Install script for arbitrary GitHub repository", body = ScriptResponse, content_type = "application/x-sh")
  ),
  tag = "install"
)]
async fn install_arbitrary_github_handler(
  Path((user, repo)): Path<(String, String)>,
  Query(mut q): Query<InstallQueryOptions>,
) -> impl IntoResponse {
  debug!(
    "install_arbitrary_github_handler({:?}, {:?}) with {:#?}",
    user, repo, q
  );
  let unsupported_app = SupportedApp::new(
    "unsupported",
    Repo::github(&format!("{}/{}", user, repo)),
    "github",
  );

  let (target, links) = load_app(&mut q, &unsupported_app).await.unwrap();
  let json_links: Vec<serde_json::Value> = links.iter().map(|x| x.json()).collect();
  let tera_context = Context::from_value(json!({
      "app": "",
      "force": false,
      "quiet": false,
      "assets": json_links,
      "log_level": q.log_level.clone()
  }))
  .unwrap();
  let (script, extension) = match target.os {
    TargetOs::Windows => (TEMPLATES.render("install.ps1", &tera_context), "ps1"),
    TargetOs::Linux => (TEMPLATES.render("install.sh", &tera_context), "sh"),
    _ => (TEMPLATES.render("install.sh", &tera_context), "sh"),
  };

  ScriptResponse::new(format!("install.{}", extension), script.unwrap())
}

#[utoipa::path(
  get,
  path = "/install/{app}",
  params(
    ("app" = String, Path, description = "Application name (e.g., yq, jq, gh)")
  ),
  responses(
    (status = 200, description = "Install script for the application", body = ScriptResponse, content_type = "application/x-sh")
  ),
  tag = "install"
)]
async fn install_handler(
  Path(app): Path<String>,
  Query(mut q): Query<InstallQueryOptions>,
) -> impl IntoResponse {
  debug!("install_handler({:?}, {:?})", app, q);
  q.set_app(app.clone());

  let supported_app = supported_apps::get_app(&app).unwrap();

  let (target, links) = load_app(&mut q, &supported_app).await.unwrap();
  let json_links: Vec<serde_json::Value> = links.iter().map(|x| x.json()).collect();
  let tera_context = Context::from_value(json!({
      "app": app,
      "force": false,
      "quiet": false,
      "assets": json_links,
      "log_level": q.log_level.clone()
  }))
  .unwrap();
  let (script, extension) = match target.os {
    TargetOs::Windows => (TEMPLATES.render("install.ps1", &tera_context), "ps1"),
    TargetOs::Linux => (TEMPLATES.render("install.sh", &tera_context), "sh"),
    _ => (TEMPLATES.render("install.sh", &tera_context), "sh"),
  };

  ScriptResponse::new(format!("install-{}.{}", app, extension), script.unwrap())
}

async fn load_app(
  q: &mut InstallQueryOptions,
  supported_app: &SupportedApp,
) -> anyhow::Result<(TargetDeployment, Vec<DownloadInfo>)> {
  let arch = q.arch.clone();
  let os = q.os.clone();
  let version = q.version.clone();
  let target_deployment = TargetDeployment::new(os, arch);
  debug!("target_deployment loaded: {:#?}", target_deployment);
  Ok((
    target_deployment.clone(),
    get_github_download_links(&supported_app.repo, &target_deployment, &version).await?,
  ))
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
    .route(
      "/install/{user}/{repo}",
      get(install_arbitrary_github_handler),
    )
    .route("/install/{app}", get(install_handler));

  let app = Router::new()
    .route("/", get(root_handler))
    .route("/favicon.ico", get(favicon))
    .nest("/v1", v1_router)
    .merge(SwaggerUi::new("/swagger-ui").url("/openapi.json", ApiDoc::openapi()))
    .layer(CorsLayer::permissive());
  app
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
    let response = server.get("v1/install/yutc").await;
    response.assert_status_ok();
    response.assert_header("Content-Type", "application/x-sh");
  }
}

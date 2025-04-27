extern crate core;
#[macro_use]
extern crate rocket;

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
use log::info;
use rocket::http::ContentType;
use rocket::response::content;
use rocket::serde::json::json;
use rocket::{Request, Response};
use rocket_okapi::settings::UrlObject;
use rocket_okapi::{openapi, openapi_get_routes, rapidoc::*};
use std::path::PathBuf;
use std::sync::LazyLock;
use std::env;
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

#[catch(404)]
fn not_found(_req: &Request) -> content::RawHtml<String> {
    let html = static_site::load_static("404.html").unwrap_or("".to_string());
    content::RawHtml(html)
}

#[openapi(skip)]
#[get("/favicon.ico")]
async fn favicon<'r>() -> (ContentType, Vec<u8>) {
    (ContentType::Icon, FAVICON.to_vec())
}

#[openapi()]
#[get("/install/<user>/<repo>?<q..>", rank = 1)]
async fn install_arbitrary_github_handler(
    user: &str,
    repo: &str,
    mut q: InstallQueryOptions,
) -> ScriptResponse {
    debug!("install_arbitrary_github_handler({:?}, {:?}) with {:#?}", user, repo, q);
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
    // StringList::new(links.unwrap())

    ScriptResponse::new(format!("install.sh"), script.unwrap())
}

#[openapi()]
#[get("/install/<app>?<q..>", rank = 1)]
async fn install_handler(app: &str, mut q: InstallQueryOptions) -> ScriptResponse {
    debug!("install_handler({:?}, {:?})", app, q);
    q.set_app(app.to_owned());

    let supported_app = supported_apps::get_app(app).unwrap();

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
    // StringList::new(links.unwrap())

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

#[openapi()]
#[get("/", rank = 10)]
async fn root_handler() -> content::RawHtml<String> {
    info!("{:?}", "root");
    info!("{:?}", TERMLIBS_ROOT);
    let html = static_site::load_static("index.html").unwrap_or("".to_string());
    content::RawHtml(html)
}

#[launch]
async fn rocket() -> _ {
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

    let figment = rocket::Config::figment()
        .merge(("port", port))
        .merge(("address", listen_ip))
        .merge(("ident", "termlibs".to_string()));
    rocket::custom(figment)
        .register("/", catchers![not_found])
        .mount(
            "/",
            openapi_get_routes![
                favicon,
                install_handler,
                root_handler,
                install_arbitrary_github_handler
            ],
        )
        .mount(
            "/rapidoc/",
            make_rapidoc(&RapiDocConfig {
                general: GeneralConfig {
                    spec_urls: vec![UrlObject::new("General", "../openapi.json")],
                    ..Default::default()
                },
                hide_show: HideShowConfig {
                    allow_spec_url_load: false,
                    allow_spec_file_load: false,
                    ..Default::default()
                },
                ..Default::default()
            }),
        )
}

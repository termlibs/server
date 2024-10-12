extern crate core;
#[macro_use]
extern crate rocket;

mod shell_files;
mod static_site;
mod types;

use crate::types::ScriptResponse;
use log::info;
use rocket::http::ContentType;
use rocket::response::content;
use rocket::Request;
use std::path::PathBuf;
use std::sync::LazyLock;
use std::env;
use types::InstallQueryOptions;

const FAVICON: &[u8] = include_bytes!("../favicon.ico");
static  TERMLIBS_ROOT: LazyLock<PathBuf> =
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
  Ok(())
}

#[catch(404)]
fn not_found(_req: &Request) -> content::RawHtml<String> {
  let html = static_site::load_static("404.html").unwrap_or("".to_string());
  content::RawHtml(html)
}

#[get("/favicon.ico")]
async fn favicon<'r>() -> (ContentType, Vec<u8>) {
  (ContentType::Icon, FAVICON.to_vec())
}

#[get("/install/<app>?<q..>", rank = 1)]
async fn install_handler(app: &str, mut q: InstallQueryOptions) -> ScriptResponse {
  info!("{:?} {:?}", app, q);
  q.set_app(app.to_owned());
  let output = shell_files::create_install_script(Some(q)).await.unwrap();
  ScriptResponse::new(format!("install-{}.sh", app), output)
}

#[get("/", rank = 10)]
async fn root_handler() -> content::RawHtml<String> {
  info!("{:?}", "root");
  let html = static_site::load_static("index.html").unwrap_or("".to_string());
  content::RawHtml(html)
}

#[launch]
async fn rocket() -> _ {
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
    .mount("/", routes![favicon, install_handler, root_handler])
}

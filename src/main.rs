#[macro_use]
extern crate rocket;
extern crate core;

mod static_site;
mod shell_files;

use comrak::Options;
use log::{error, info};
use rocket::futures::io::Cursor;
use rocket::http::ContentType;
use rocket::response::{content, Responder};
use rocket::serde::Deserialize;
use rocket::{response, Request, Response};
use std::convert::Infallible;
use std::net::SocketAddr;
use std::ops::Deref;
use std::path::PathBuf;
use std::{env, error};
use tokio::net::TcpListener;

fn setup_logger(log_level: &str) -> Result<(), fern::InitError> {
  let log_level = log_level.to_uppercase();
  let level = match log_level.as_str() {
    "DEBUG" => log::LevelFilter::Debug,
    "INFO" => log::LevelFilter::Info,
    "WARN" => log::LevelFilter::Warn,
    "ERROR" => log::LevelFilter::Error,
    _ => log::LevelFilter::Info
  };
  let _ = fern::Dispatch::new().format(
    |out, message, record| {
      out.finish(format_args!(
        "{} {}: {}",
        chrono::Local::now().format("[%Y-%m-%d %H:%M:%S]"),
        record.level(),
        message
      ));
    }
  ).level(level).chain(std::io::stdout()).apply()?;
  Ok(())
}

#[catch(404)]
fn not_found(_req: &Request) -> content::RawHtml<String> {
  let html = static_site::load_static("404.html").unwrap_or("".to_string());
  let response = content::RawHtml(html);
  response
}

#[derive(Debug, PartialEq, FromForm)]
struct InstallQueryOptions {
  version: Option<String>,
  prefix: Option<String>,
}

impl InstallQueryOptions {
  fn version(&self) -> String {
    self.version.clone().unwrap_or("latest".to_string())
  }

  fn prefix(&self) -> String {
    self.prefix.clone().unwrap_or("$HOME/.local".to_string())
  }

  fn to_args(&self) -> String {
    format!("--version=\"{}\" --prefix=\"{}\"", self.version(), self.prefix())
  }

  fn is_none(&self) -> bool {
    self.prefix.clone().is_none() && self.version.clone().is_none()
  }

  fn is_some(&self) -> bool {
    !self.is_none().clone()
  }
}

#[get("/install/<app>?<q..>", rank = 1)]
async fn install_handler(app: &str, q: InstallQueryOptions) -> String {
  info!("{:?}", app);
  info!("{:?}", q);
  let version = q.version();
  let prefix = q.prefix();
  let args = if q.is_some() { Some(q.to_args()) } else { None };
  let output = shell_files::open_file("install.sh/scripts/install_all.sh", args).await.unwrap();
  output
}

#[get("/", rank = 10)]
async fn root_handler() -> content::RawHtml<String> {
  info!("{:?}", "root");
  let html = static_site::load_static("index.html").unwrap_or("".to_string());
  let response = content::RawHtml(html);
  response
}

#[launch]
async fn rocket() -> _ {
  let port = env::var("PORT").unwrap_or("8080".to_string()).parse::<u16>().unwrap();
  let log_level = env::var("LOG_LEVEL").unwrap_or("DEBUG".to_string());
  let listen_ip: String = env::var("LISTEN_IP").unwrap_or("0.0.0.0".to_string());

  let _ = setup_logger(log_level.as_str()).unwrap_or(());

  info!("starting server at {:?}:{}", listen_ip, port);

  let figment = rocket::Config::figment()
    .merge(("port", port))
    .merge(("address", listen_ip))
    .merge(("ident", "termlibs".to_string()));
  rocket::custom(figment).register("/", catchers![not_found]).mount(
    "/", routes![
      install_handler,
      root_handler
    ],
  )
}

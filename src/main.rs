#[macro_use]
extern crate rocket;
mod static_site;

use log::{error, info};
use rocket::{response, Request, Response};
use std::convert::Infallible;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::{env, error};
use std::ops::Deref;
use rocket::futures::io::Cursor;
use rocket::http::ContentType;
use rocket::response::{content, Responder};
use rocket::serde::Deserialize;
use tokio::net::TcpListener;
use url::Url;

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
fn not_found(req: &Request) -> content::RawHtml<String> {
  info!("{:?}", req);
  let response = content::RawHtml("404".to_string());
  response
}

#[get("/<extra..>")]
async fn root_handler(extra: PathBuf) -> content::RawHtml<String> {
  info!("{:?}", extra);
  let html = static_site::load_static("/").unwrap_or("".to_string());
  let response = content::RawHtml(html);
  response
}

#[launch]
async fn rocket() -> _ {
  let port = env::var("PORT").unwrap_or("8081".to_string()).parse::<u16>().unwrap();
  let log_level = env::var("LOG_LEVEL").unwrap_or("DEBUG".to_string());
  let listen_ip: String = env::var("LISTEN_IP").unwrap_or("0.0.0.0".to_string());

  let _ = setup_logger(log_level.as_str()).unwrap_or(());

  info!("starting server at {:?}:{}", listen_ip, port);

  let figment = rocket::Config::figment()
    .merge(("port", port))
    .merge(("address", listen_ip))
    .merge(("ident", "termlibs".to_string()));
  rocket::custom(figment).mount(
    "/", routes![
      // not_found,
      root_handler
    ],
  )
}

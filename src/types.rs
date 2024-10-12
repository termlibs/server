use rocket::http::hyper::header::CONTENT_DISPOSITION;
use rocket::http::{ContentType, Status};
use rocket::response::Responder;
use rocket::{http, response, Request, Response};
use std::io::Cursor;

pub(crate) trait QueryOptions {
  fn to_args(&self) -> String;
}

#[derive(Debug, PartialEq, FromForm)]
pub struct InstallQueryOptions {
  version: Option<String>,
  prefix: Option<String>,
  app: Option<String>,
}

impl QueryOptions for InstallQueryOptions {
  fn to_args(&self) -> String {
    format!(
      "--version=\"{}\" --prefix=\"{}\" \"{}\"",
      self.version(),
      self.prefix(),
      self.app()
    )
  }
}

impl InstallQueryOptions {
  pub(crate) fn version(&self) -> String {
    self.version.clone().unwrap_or("latest".to_string())
  }

  pub(crate) fn prefix(&self) -> String {
    self.prefix.clone().unwrap_or("$HOME/.local".to_string())
  }

  pub(crate) fn app(&self) -> String {
    self.app.clone().unwrap()
  }

  #[allow(dead_code)]
  pub(crate) fn is_none(&self) -> bool {
    self.prefix.clone().is_none() && self.version.clone().is_none()
  }
  #[allow(dead_code)]
  pub(crate) fn is_some(&self) -> bool {
    !self.is_none().clone()
  }

  pub(crate) fn set_app(&mut self, app: String) {
    self.app = Some(app);
  }
}

pub(crate) struct ScriptResponse {
  filename: String,
  body: Cursor<Vec<u8>>,
  body_size: usize,
}

impl ScriptResponse {
  pub(crate) fn new(filename: String, body: String) -> ScriptResponse {
    let body = body.into_bytes();
    let body_size = body.len();
    let body = Cursor::new(body);

    ScriptResponse {
      filename,
      body,
      body_size,
    }
  }
}

impl<'r> Responder<'r, 'static> for ScriptResponse {
  fn respond_to(self, _req: &Request) -> response::Result<'static> {
    let content_type = ContentType::new("application", "x-sh");
    Response::build()
      .status(Status::Ok)
      .header(content_type)
      .sized_body(self.body_size, self.body)
      .header(http::Header::new(
        CONTENT_DISPOSITION.as_str(),
        format!("inline; filename=\"{}\"", self.filename),
      ))
      .ok()
  }
}

use axum::body::Body;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Serialize, Deserialize, ToSchema)]
pub(crate) struct ScriptResponse {
  filename: String,
  #[serde(skip)]
  body: Body,
  #[serde(skip)]
  shell_name: String,
  body_size: usize,
}

impl ScriptResponse {
  pub(crate) fn new(filename: String, body: String) -> ScriptResponse {
    let body = body.into_bytes();
    let body_size = body.len();
    let body: Body = body.into();

    let shell_name = match filename.split('.').last().unwrap() {
      "sh" => "sh",
      "ps1" => "powershell",
      _ => "sh",
    }
    .to_string();

    ScriptResponse {
      filename,
      body,
      body_size,
      shell_name,
    }
  }
}

impl IntoResponse for ScriptResponse {
  fn into_response(self) -> Response {
    Response::builder()
      .status(StatusCode::OK)
      .header("Content-Type", format!("application/x-{}", self.shell_name))
      .header(
        "Content-Disposition",
        format!("inline; filename=\"{}\"", self.filename),
      )
      .body(self.body)
      .unwrap()
  }
}

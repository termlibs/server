use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Serialize;

#[derive(Debug)]
pub(crate) enum AppError {
  InvalidInput(String),
  UnsupportedApp(String),
  NoMatchingAssets { repo: String, target: String },
  UpstreamGithub(String),
  Template(String),
}

impl AppError {
  fn status_code(&self) -> StatusCode {
    match self {
      AppError::InvalidInput(_) => StatusCode::BAD_REQUEST,
      AppError::UnsupportedApp(_) => StatusCode::NOT_FOUND,
      AppError::NoMatchingAssets { .. } => StatusCode::NOT_FOUND,
      AppError::UpstreamGithub(message) => {
        if message.to_ascii_lowercase().contains("rate limit") {
          StatusCode::SERVICE_UNAVAILABLE
        } else {
          StatusCode::BAD_GATEWAY
        }
      }
      AppError::Template(_) => StatusCode::INTERNAL_SERVER_ERROR,
    }
  }

  fn code(&self) -> &'static str {
    match self {
      AppError::InvalidInput(_) => "invalid_input",
      AppError::UnsupportedApp(_) => "unsupported_app",
      AppError::NoMatchingAssets { .. } => "no_matching_assets",
      AppError::UpstreamGithub(_) => "upstream_github_error",
      AppError::Template(_) => "template_error",
    }
  }

  fn message(&self) -> String {
    match self {
      AppError::InvalidInput(message) => message.clone(),
      AppError::UnsupportedApp(app) => format!("Unsupported app: {}", app),
      AppError::NoMatchingAssets { repo, target } => {
        format!(
          "No matching assets found for '{}' and target '{}'",
          repo, target
        )
      }
      AppError::UpstreamGithub(message) => message.clone(),
      AppError::Template(message) => message.clone(),
    }
  }
}

#[derive(Serialize)]
struct ErrorBody {
  error: &'static str,
  message: String,
}

impl IntoResponse for AppError {
  fn into_response(self) -> Response {
    let status = self.status_code();
    let body = Json(ErrorBody {
      error: self.code(),
      message: self.message(),
    });
    (status, body).into_response()
  }
}

impl From<tera::Error> for AppError {
  fn from(value: tera::Error) -> Self {
    Self::Template(value.to_string())
  }
}

impl From<octocrab::Error> for AppError {
  fn from(value: octocrab::Error) -> Self {
    Self::UpstreamGithub(value.to_string())
  }
}

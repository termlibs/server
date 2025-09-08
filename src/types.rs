pub(crate) use crate::app_downloader::{TargetArch, TargetOs};
use crate::supported_apps::Repo;
use axum::body::Body;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use std::fmt::Display;
use utoipa::{IntoParams, ToSchema};

pub(crate) trait QueryOptions {
    fn to_args(&self) -> String;
}

#[derive(Debug, PartialEq, Clone, Deserialize, Serialize, ToSchema)]
pub enum InstallMethod {
    Installer,
    Binary,
}

impl Display for InstallMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InstallMethod::Installer => write!(f, "{}", "installer"),
            InstallMethod::Binary => write!(f, "{}", "binary"),
        }
    }
}

impl From<&str> for InstallMethod {
    fn from(value: &str) -> Self {
        match value {
            "installer" => InstallMethod::Installer,
            _ => InstallMethod::Binary,
        }
    }
}

#[derive(Debug, PartialEq, Clone, Deserialize, Serialize, ToSchema, IntoParams)]
pub struct InstallQueryOptions {
    app: Option<String>,
    #[serde(default = "default_latest")]
    pub(crate) version: String,
    #[serde(default = "default_prefix")]
    prefix: String,
    #[serde(default = "default_arch")]
    pub(crate) arch: TargetArch,
    #[into_params(names("os"))]
    #[serde(default = "default_os")]
    pub(crate) os: TargetOs,
    #[serde(default = "default_method")]
    method: InstallMethod,
    #[serde(default = "default_download_only")]
    download_only: bool,
    #[serde(default = "default_force")]
    force: bool,
    #[serde(default = "default_quiet")]
    quiet: bool,
    #[serde(default = "default_log_level")]
    pub(crate) log_level: String,
}

fn default_latest() -> String {
    "latest".to_string()
}

fn default_prefix() -> String {
    "$HOME/.local".to_string()
}

fn default_arch() -> TargetArch {
    TargetArch::Amd64
}

fn default_os() -> TargetOs {
    TargetOs::Linux
}

fn default_method() -> InstallMethod {
    InstallMethod::Binary
}

fn default_download_only() -> bool {
    false
}

fn default_force() -> bool {
    false
}

fn default_quiet() -> bool {
    false
}

fn default_log_level() -> String {
    "DEBUG".to_string()
}

impl InstallQueryOptions {
    pub(crate) fn set_app(&mut self, app: String) {
        self.app = Some(app);
    }

    pub fn template_globals(&self) -> Map<String, Value> {
        json!({
            "app": self.app.clone(),
            "version": self.version.as_str(),
            "prefix": self.prefix.as_str(),
            "arch": self.arch.to_string(),
            "os": self.os.to_string(),
            "method": self.method.to_string(),
            "download_only": self.download_only,
            "force": self.force,
            "quiet": self.quiet,
            "log_level": self.log_level.as_str(),
        })
        .as_object()
        .unwrap()
        .to_owned()
    }
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct StringList {
    links: Vec<String>,
}

impl StringList {
    pub fn new(links: Vec<String>) -> StringList {
        StringList { links }
    }
}

impl IntoResponse for StringList {
    fn into_response(self) -> Response {
        let body = Body::from(serde_json::to_vec_pretty(&self.links).unwrap());
        Response::builder()
            .status(StatusCode::OK)
            .header("Content-Type", "application/json")
            .body(body)
            .unwrap()
    }
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct ScriptResponse {
    filename: String,
    #[serde(skip)]
    body: Body,
    body_size: usize,
}

impl QueryOptions for InstallQueryOptions {
    fn to_args(&self) -> String {
        "".to_ascii_lowercase()
    }
}

impl ScriptResponse {
    pub(crate) fn new(filename: String, body: String) -> ScriptResponse {
        let body = body.into_bytes();
        let body_size = body.len();
        let body: Body = body.into();

        ScriptResponse {
            filename,
            body,
            body_size,
        }
    }
}

impl IntoResponse for ScriptResponse {
    fn into_response(self) -> Response {
        Response::builder()
            .status(StatusCode::OK)
            .header("Content-Type", "application/x-sh")
            .header(
                "Content-Disposition",
                format!("inline; filename=\"{}\"", self.filename),
            )
            .body(self.body)
            .unwrap()
    }
}

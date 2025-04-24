use crate::app_downloader::{TargetArch, TargetOs};
use rocket::http::hyper::header::CONTENT_DISPOSITION;
use rocket::http::{ContentType, Status};
use rocket::response::Responder;
use rocket::yansi::Paint;
use rocket::{http, response, Request, Response};
use rocket_okapi::gen::OpenApiGenerator;
use rocket_okapi::okapi::openapi3::Responses;
use rocket_okapi::response::OpenApiResponder;
use rocket_okapi::JsonSchema;
use std::fmt::Display;
use std::io;
use std::io::Cursor;

pub(crate) trait QueryOptions {
    fn to_args(&self) -> String;
}

#[derive(Debug, PartialEq, Clone, FromFormField, JsonSchema)]
pub enum InstallMethod {
    Installer,
    Binary,
}

impl Display for InstallMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InstallMethod::Installer => write!(f, "{}", Paint::yellow("installer")),
            InstallMethod::Binary => write!(f, "{}", Paint::yellow("binary")),
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

#[derive(Debug, PartialEq, Clone, FromForm, JsonSchema)]
pub struct InstallQueryOptions {
    app: Option<String>,
    #[field(default = "latest")]
    pub(crate) version: String,
    #[field(default = "$HOME/.local")]
    prefix: String,
    #[field(default = "amd64")]
    pub(crate) arch: TargetArch,
    #[field(default = "linux")]
    pub(crate) os: TargetOs,
    #[field(default = "binary")]
    method: InstallMethod,
    #[field(default = false)]
    download_only: bool,
    #[field(default = false)]
    force: bool,
    #[field(default = false)]
    quiet: bool,
    #[field(default = "DEBUG")]
    log_level: String
}

impl InstallQueryOptions {
    pub(crate) fn set_app(&mut self, app: String) {
        self.app = Some(app);
    }

    pub fn template_globals(&self) -> liquid::Object {
        liquid::object!({
            "app": self.app.as_ref().unwrap(),
            "version": self.version.as_str(),
            "prefix": self.prefix.as_str(),
            "arch": self.arch.to_string(),
            "os": self.os.to_string(),
            "method": self.method.to_string(),
            "download_only": self.download_only,
            "force": self.force,
            "quiet": self.quiet,
            "log_level": self.log_level.as_str(),
            "file_url": format!("https://github.com/mikefarah/yq/releases/download/v4.30.2/yq_{}_{}", self.os, self.arch)
        })
    }
}

#[derive(JsonSchema)]
pub struct StringList {
    links: Vec<String>,
}

impl StringList {
    pub fn new(links: Vec<String>) -> StringList {
        StringList { links }
    }
}

impl<'r> OpenApiResponder<'r, 'static> for StringList {
    fn responses(gen: &mut OpenApiGenerator) -> rocket_okapi::Result<Responses> {
        Ok(Responses::default())
    }
}

impl<'r> Responder<'r, 'static> for StringList {
    fn respond_to(self, _req: &Request) -> response::Result<'static> {
        let content_type = ContentType::new("application", "json");
        let data = serde_json::to_string(self.links.as_slice()).unwrap();
        Response::build()
            .status(Status::Ok)
            .header(content_type)
            .sized_body(data.len(), Cursor::new(data))
            .ok()
    }
}
#[derive(JsonSchema)]
pub struct ScriptResponse {
    filename: String,
    #[schemars(skip)]
    body: Cursor<Vec<u8>>,
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
        let body = Cursor::new(body);

        ScriptResponse {
            filename,
            body,
            body_size,
        }
    }
}

impl<'r> OpenApiResponder<'r, 'static> for ScriptResponse {
    fn responses(gen: &mut OpenApiGenerator) -> rocket_okapi::Result<Responses> {
        Ok(Responses::default())
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

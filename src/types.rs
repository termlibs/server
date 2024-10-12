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
        let mut args_out: Vec<String> = vec![];
        if let Some(v) = self.version.clone() {
            args_out.push(format!("--version={}", v));
        }
        if let Some(p) = self.prefix.clone() {
            args_out.push(format!("--prefix={}", p));
        }
        args_out.push(self.app.clone().unwrap());
        args_out.join(" ")
    }
}

impl InstallQueryOptions {
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

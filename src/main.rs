mod static_site;

use std::convert::Infallible;
use std::error;
use std::net::{SocketAddr};
use hyper::body::Bytes;
use http_body_util::Full;
use hyper::{Request, Response};
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper_util::rt::TokioIo;
use log::{error, info};
use tokio::net::TcpListener;
use url::Url;

async fn root_handler(req: Request<hyper::body::Incoming>) -> Result<Response<Full<Bytes>>, Infallible> {
  println!("{:?}", req);
  let html= static_site::load_static("/");
  match html {
    Some(html) => {
      Ok(
        Response::new(
          Full::new(
            Bytes::from(html)
          )
        )
      )
    }
    None => Ok(Response::new(Full::new(Bytes::from("err"))))
  }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn error::Error + Send + Sync>> {
  let ip4 = [127, 0, 0, 1];
  let port = 9090;

  info!("starting server at {:?}:{}", ip4, port);
  let addr = SocketAddr::from((ip4, port));
  let tcp = TcpListener::bind(addr).await?;

  loop {
    let (stream, _) = tcp.accept().await?;

    let io = TokioIo::new(stream);
    tokio::task::spawn(
      async move {
        if let Err(err) = http1::Builder::new().serve_connection(io, service_fn(root_handler)).await {
          error!("unable to handle connection {:?}", err)
        }
      }
    );
  }
}

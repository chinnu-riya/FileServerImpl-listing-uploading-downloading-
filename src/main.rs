
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Method, Request, Response, Server, StatusCode};
use std::fs::File;
use std::io::prelude::*;
use tokio::fs;
use tokio_util::codec::{Bytes, FramedRead};
use tokio_util::compat::Tokio02AsyncReadCompatExt;

async fn list_files(_req: Request<Body>) -> Result<Response<Body>, hyper::Error> {
    let dir = std::fs::read_dir(".").map_err(|_| hyper::Error::from(std::io::Error::new(
        std::io::ErrorKind::Other,
        "Failed to read directory",
    )))?;

    let mut body = String::new();
    for entry in dir {
        if let Ok(entry) = entry {
            if let Ok(file_name) = entry.file_name().into_string() {
                body.push_str(&format!("{}\n", file_name));
            }
        }
    }

    Ok(Response::new(Body::from(body)))
}

async fn download_file(req: Request<Body>) -> Result<Response<Body>, hyper::Error> {
    let path = req.uri().path();
    let file_name = path.trim_start_matches('/');

    let file = fs::File::open(file_name)
        .await
        .map_err(|_| hyper::Error::from(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "File not found",
        )))?;

    let reader = FramedRead::new(file.compat(), Bytes::default());

    Ok(Response::new(Body::wrap_stream(reader.compat())))
}

async fn upload_file(mut req: Request<Body>) -> Result<Response<Body>, hyper::Error> {
    let path = req.uri().path();
    let file_name = path.trim_start_matches('/');

    let mut file = File::create(file_name).map_err(|_| hyper::Error::from(std::io::Error::new(
        std::io::ErrorKind::Other,
        "Failed to create file",
    )))?;

    while let Some(chunk) = req.body_mut().data().await {
        let chunk = chunk.map_err(|_| hyper::Error::from(std::io::Error::new(
            std::io::ErrorKind::Other,
            "Failed to read request body",
        )))?;

        file.write_all(&chunk)
            .map_err(|_| hyper::Error::from(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Failed to write to file",
            )))?;
    }

    Ok(Response::new(Body::empty()))
}

async fn handle_request(req: Request<Body>) -> Result<Response<Body>, hyper::Error> {
    match (req.method(), req.uri().path()) {
        (&Method::GET, "/list") => list_files(req).await,
        (&Method::GET, _) => download_file(req).await,
        (&Method::POST, _) => upload_file(req).await,
        _ => Ok(Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Body::empty())
            .unwrap()),
    }
}

#[tokio::main]
async fn main() {
    let addr = ([127, 0, 0, 1], 3000).into();
    let make_svc = make_service_fn(|_conn| async { Ok::<_, hyper::Error>(service_fn(handle_request)) });
    let server = Server::bind(&addr).serve(make_svc);

    println!("Server running on http://{}", addr);

    if let Err(e) = server.await {
        eprintln!("Server error: {}", e);
    }
          }
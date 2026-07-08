use hyper::body::Incoming;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Request, Response};
use hyper_util::rt::TokioIo;
use http_body_util::Full;
use std::convert::Infallible;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::net::TcpListener;

type Body = Full<hyper::body::Bytes>;

/// Serve static files from a base directory.
async fn serve_static(path: PathBuf, base_dir: PathBuf) -> Result<Response<Body>, Infallible> {
    let resolved = base_dir.join(&path);

    // Prevent directory traversal
    if !resolved.starts_with(&base_dir) {
        return Ok(Response::builder()
            .status(403)
            .header("content-type", "text/plain")
            .body(Full::from(hyper::body::Bytes::from("403 Forbidden")))
            .unwrap());
    }

    // Default to index.html for directories
    let final_path = if resolved.is_dir() {
        resolved.join("index.html")
    } else {
        resolved
    };

    // Read the file
    let data = match tokio::fs::read(&final_path).await {
        Ok(data) => data,
        Err(_) => {
            return Ok(Response::builder()
                .status(404)
                .header("content-type", "text/plain")
                .body(Full::from(hyper::body::Bytes::from("404 Not Found")))
                .unwrap());
        }
    };

    let mime = mime_guess::from_path(&final_path)
        .first_or_text_plain()
        .to_string();

    let response = Response::builder()
        .status(200)
        .header("content-type", mime)
        .body(Full::from(hyper::body::Bytes::from(data)))
        .unwrap();

    Ok(response)
}

async fn handle_request(req: Request<Incoming>, base_dir: Arc<PathBuf>) -> Result<Response<Body>, Infallible> {
    let path_str = req.uri().path().to_string();

    // Normalize: strip leading slash, default to index.html
    let path = if path_str == "/" || path_str.is_empty() {
        PathBuf::from("index.html")
    } else {
        PathBuf::from(path_str.trim_start_matches('/'))
    };

    serve_static(path, (*base_dir).clone()).await
}

#[tokio::main]
async fn main() {
    let port = std::env::var("PORT").unwrap_or_else(|_| "8080".to_string());
    let addr: SocketAddr = format!("0.0.0.0:{}", port).parse().expect("invalid address");
    let base_dir = Arc::new(std::env::current_dir().expect("couldn't get cwd"));

    println!("🚀 Serving static files from: {}", base_dir.display());
    println!("📡 Open http://localhost:{}", port);

    let listener = TcpListener::bind(addr).await.expect("couldn't bind to address");

    loop {
        let (stream, _) = listener.accept().await.expect("accept failed");
        let io = TokioIo::new(stream);
        let base = base_dir.clone();

        tokio::spawn(async move {
            if let Err(e) = http1::Builder::new()
                .serve_connection(io, service_fn(move |req| handle_request(req, base.clone())))
                .await
            {
                eprintln!("Error serving connection: {}", e);
            }
        });
    }
}

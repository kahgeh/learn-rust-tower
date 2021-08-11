use std::{
    convert::Infallible,
    net::SocketAddr,
    task::Poll,
    time::{Duration, Instant},
};

use futures::{
    future::{ready, Ready},
    Future,
};
use hyper::{
    service::{make_service_fn, service_fn},
    Body, Request, Response, Server,
};
use pin_project::pin_project;
use tokio::time::sleep;
use tower::Service;
use tracing::info;
use tracing_subscriber::fmt;

async fn handle(_req: Request<Body>) -> Result<Response<Body>, Infallible> {
    sleep(Duration::from_secs(5)).await;
    Ok(Response::new(Body::from("HelloWorld")))
}

#[tokio::main]
async fn main() {
    fmt::init();
    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    //let svc = Logging::new(HelloWorld);
    let svc = service_fn(handle);
    let svc = Logging::new(svc);

    let make_service = make_service_fn(|_con| async move { Ok::<_, Infallible>(svc) });
    let server = Server::bind(&addr).serve(make_service);

    if let Err(e) = server.await {
        eprintln!("server error :{}", e);
    }
}

#[derive(Clone, Copy)]
struct HelloWorld;

impl Service<Request<Body>> for HelloWorld {
    type Response = Response<Body>;

    type Error = Infallible;

    type Future = Ready<Result<Self::Response, Self::Error>>;

    fn poll_ready(
        &mut self,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, _req: Request<Body>) -> Self::Future {
        ready(Ok(Response::new(Body::from("HelloWorld"))))
    }
}

#[derive(Clone, Copy)]
struct Logging<S> {
    inner: S,
}

impl<S> Logging<S> {
    fn new(inner: S) -> Self {
        Self { inner }
    }
}

impl<S, B> Service<Request<B>> for Logging<S>
where
    S: Service<Request<B>> + Clone + Send + 'static,
    B: Send + 'static,
    S::Future: Send,
{
    type Response = S::Response;

    type Error = S::Error;

    type Future = LoggingFuture<S::Future>;

    fn poll_ready(&mut self, cx: &mut std::task::Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<B>) -> Self::Future {
        let method = req.method().clone();
        let path = req.uri().path().to_string();
        let start = Instant::now();
        let f = LoggingFuture {
            future: self.inner.call(req),
            method,
            path,
            start,
        };

        f
    }
}

#[pin_project]
struct LoggingFuture<F> {
    #[pin]
    future: F,
    path: String,
    method: hyper::Method,
    start: Instant,
}

impl<F> Future for LoggingFuture<F>
where
    F: Future,
{
    type Output = F::Output;

    fn poll(self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        info!("start processing request {} {}", this.method, this.path);

        let res = match this.future.poll(cx) {
            Poll::Ready(res) => res,
            Poll::Pending => return Poll::Pending,
        };

        info!(
            "complete processing request {} {} in {:?}",
            this.method,
            this.path,
            this.start.elapsed()
        );

        Poll::Ready(res)
    }
}

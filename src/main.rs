use std::{convert::Infallible, net::SocketAddr, task::Poll};

use futures::future::{ready, BoxFuture, Ready};

use hyper::{service::make_service_fn, Body, Request, Response, Server};
use tower::Service;
use tracing::info;
use tracing_subscriber::fmt;

#[tokio::main]
async fn main() {
    fmt::init();
    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    let svc = Logging::new(HelloWorld);

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

    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, cx: &mut std::task::Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<B>) -> Self::Future {
        let mut inner = self.inner.clone();
        Box::pin(async move {
            let method = req.method().clone();
            let path = req.uri().path().to_string();
            info!("start processing request {} {}", method, path);
            let response = inner.call(req).await;
            info!("complete processing request {} {}", method, path);
            response
        })
    }
}

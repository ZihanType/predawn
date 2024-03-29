use self::private::TracingHandler;
use super::Middleware;
use crate::handler::Handler;

#[derive(Clone, Copy)]
pub struct Tracing;

impl<H: Handler> Middleware<H> for Tracing {
    type Output = TracingHandler<H>;

    fn transform(self, input: H) -> Self::Output {
        TracingHandler { inner: input }
    }
}

mod private {
    use std::time::Instant;

    use predawn_core::{error::Error, request::Request, response::Response};
    use tracing::Instrument;

    use crate::handler::Handler;

    pub struct TracingHandler<H> {
        pub inner: H,
    }

    impl<H: Handler> Handler for TracingHandler<H> {
        async fn call(&self, req: Request) -> Result<Response, Error> {
            let head = &req.head;

            let span = ::tracing::info_span!(
                target: module_path!(),
                "request",
                remote_addr = %head.remote_addr(),
                version = ?head.version,
                method = %head.method,
                uri = %head.original_uri(),
            );

            async move {
                let now = Instant::now();
                let result = self.inner.call(req).await;
                let duration = now.elapsed();

                match &result {
                    Ok(response) => {
                        ::tracing::info!(
                            status = %response.status(),
                            duration = ?duration,
                            "response"
                        )
                    }
                    Err(error) => {
                        ::tracing::info!(
                            status = %error.status(),
                            duration = ?duration,
                            error = %error,
                            "error"
                        )
                    }
                };

                result
            }
            .instrument(span)
            .await
        }
    }
}

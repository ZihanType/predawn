use std::{
    convert::Infallible,
    future::{self, Future},
    io,
    net::SocketAddr,
    sync::Arc,
    time::Duration,
};

use futures_util::{pin_mut, FutureExt};
use hyper::{body::Incoming, service::service_fn};
use hyper_util::{
    rt::{TokioExecutor, TokioIo},
    server::conn::auto::Builder,
};
use predawn_core::{body::ResponseBody, request::Request};
use tokio::{
    net::{TcpListener, TcpStream},
    sync::watch::{self, Receiver, Sender},
};
use tracing::{error, info, trace};

use crate::handler::Handler;

pub struct Server {
    tcp_listener: TcpListener,
}

impl Server {
    pub fn new(tcp_listener: TcpListener) -> Self {
        Self { tcp_listener }
    }

    pub async fn run<H>(self, handler: H) -> io::Result<()>
    where
        H: Handler,
    {
        self.run_with_graceful_shutdown(handler, future::pending::<()>())
            .await
    }

    pub async fn run_with_graceful_shutdown<H, S>(self, handler: H, signal: S) -> io::Result<()>
    where
        H: Handler,
        S: Future<Output = ()> + Send + 'static,
    {
        let Self { tcp_listener } = self;
        let local_addr = tcp_listener.local_addr()?;
        let handler = Arc::new(handler);

        info!("listening {}", local_addr);

        let (signal_sender, signal_receiver) = {
            let (sender, receiver) = watch::channel(());
            (Arc::new(sender), receiver)
        };

        tokio::spawn(async move {
            signal.await;
            trace!("received graceful shutdown signal. Telling tasks to shutdown");
            drop(signal_receiver);
        });

        let (close_sender, close_receiver) = watch::channel(());

        loop {
            tokio::select! {
                conn = tcp_accept(&tcp_listener) => {
                    match conn {
                        Some((tcp_stream, remote_addr)) => handle_conn(
                            tcp_stream,
                            local_addr,
                            remote_addr,
                            signal_sender.clone(),
                            close_receiver.clone(),
                            handler.clone()
                        )
                        .await,
                        None => continue,
                    }
                }
                _ = signal_sender.closed() => {
                    trace!("signal received, not accepting new connections");
                    break;
                }

            }
        }

        drop(close_receiver);
        drop(tcp_listener);

        trace!(
            "waiting for {} task(s) to finish",
            close_sender.receiver_count()
        );
        close_sender.closed().await;

        Ok(())
    }
}

fn is_connection_error(e: &io::Error) -> bool {
    matches!(
        e.kind(),
        io::ErrorKind::ConnectionRefused
            | io::ErrorKind::ConnectionAborted
            | io::ErrorKind::ConnectionReset
    )
}

async fn tcp_accept(listener: &TcpListener) -> Option<(TcpStream, SocketAddr)> {
    match listener.accept().await {
        Ok(conn) => Some(conn),
        Err(e) => {
            if is_connection_error(&e) {
                return None;
            }

            error!("accept error: {e}");
            tokio::time::sleep(Duration::from_secs(1)).await;
            None
        }
    }
}

async fn handle_conn<H: Handler + Clone>(
    tcp_stream: TcpStream,
    local_addr: SocketAddr,
    remote_addr: SocketAddr,
    signal_sender: Arc<Sender<()>>,
    close_receiver: Receiver<()>,
    handler: H,
) {
    let tcp_stream = TokioIo::new(tcp_stream);

    trace!("connection {remote_addr} accepted");

    tokio::spawn(async move {
        let builder = Builder::new(TokioExecutor::new());
        let conn = builder.serve_connection_with_upgrades(
            tcp_stream,
            service_fn(|request: http::Request<Incoming>| {
                let handler = handler.clone();
                async move {
                    Ok::<http::Response<ResponseBody>, Infallible>(
                        handler
                            .call(Request::new(request, local_addr, remote_addr))
                            .await
                            .unwrap_or_else(|e| e.response()),
                    )
                }
            }),
        );
        pin_mut!(conn);

        let signal_closed = signal_sender.closed().fuse();
        pin_mut!(signal_closed);

        loop {
            tokio::select! {
                result = conn.as_mut() => {
                    if let Err(err) = result {
                        error!("failed to serve connection: {err:#}");
                    }
                    break;
                }
                _ = &mut signal_closed => {
                    trace!("signal received in task, starting graceful shutdown");
                    conn.as_mut().graceful_shutdown();
                }
            }
        }

        trace!("connection {remote_addr} closed");

        drop(close_receiver);
    });
}

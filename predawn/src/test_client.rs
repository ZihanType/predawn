use std::net::SocketAddr;

use reqwest::{redirect::Policy, Client, RequestBuilder};
use rudi::Context;
use tokio::net::TcpListener;

use crate::{
    app::{create_app, Hooks},
    environment::Environment,
    server::Server,
};

macro_rules! impl_request_methods {
    ($($name:ident),+ $(,)?) => {
        $(
            pub fn $name(&self, url: &str) -> RequestBuilder {
                self.client.$name(format!("http://{}{}", self.addr, url))
            }
        )+
    };
}

pub struct TestClient {
    client: Client,
    addr: SocketAddr,
    #[allow(dead_code)]
    cx: Context,
}

impl TestClient {
    impl_request_methods![get, post, put, delete, head, patch];

    pub async fn new<H: Hooks>() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        tracing::info!("listening on {}", addr);

        let (cx, router) = create_app::<H>(Environment::Test).await;

        tokio::spawn(async move {
            Server::new(listener).run(router).await.unwrap();
        });

        let client = Client::builder().redirect(Policy::none()).build().unwrap();

        Self { client, addr, cx }
    }
}

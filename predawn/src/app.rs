use std::{
    collections::{BTreeMap, HashMap},
    future::Future,
    io,
    net::SocketAddr,
    sync::Arc,
};

use config::ConfigError;
use predawn_core::{
    openapi::{Components, Info, OpenAPI, Paths},
    request::RequestBodyLimit,
};
use rudi::Context;
use tokio::{net::TcpListener, signal};

use crate::{
    config::{logger::LoggerConfig, server::ServerConfig, Config},
    controller::Controller,
    environment::Environment,
    handler::{Handler, HandlerExt},
    plugin::Plugin,
    route::{MethodRouter, Router},
    server::Server,
};

pub trait Hooks {
    fn openapi_info() -> Info {
        Default::default()
    }

    fn load_config() -> Result<Config, ConfigError> {
        Config::load(Environment::resolve_from_env())
    }

    fn init_logger(config: &Config) {
        let cfg = config.get::<LoggerConfig>().unwrap_or_default();

        tracing_subscriber::fmt()
            .with_max_level(tracing::Level::from(cfg.level))
            .init();
    }

    fn create_context(config: Config) -> impl Future<Output = Context> {
        async {
            Context::options()
                .singleton(config)
                .auto_register_async()
                .await
        }
    }

    fn after_routes(router: &Router) {
        let _router = router;
    }

    fn add_middlewares<H: Handler>(
        router: H,
        cx: &mut Context,
    ) -> impl Future<Output = impl Handler> {
        async {
            let _cx = cx;
            router
        }
    }

    fn start_server<H: Handler>(
        router: H,
        cx: &mut Context,
    ) -> impl Future<Output = io::Result<()>> {
        async {
            let cfg = cx.resolve::<ServerConfig>();

            let socket_addr = SocketAddr::new(cfg.ip, cfg.port);

            let listener = TcpListener::bind(socket_addr).await?;

            Server::new(listener)
                .run_with_graceful_shutdown(router, async {
                    let _ = signal::ctrl_c().await;
                })
                .await
        }
    }
}

pub async fn run_app<H: Hooks>() {
    let config = H::load_config().unwrap();

    H::init_logger(&config);

    let config = H::load_config().unwrap();

    let server_cfg = config.get::<ServerConfig>().unwrap_or_default();
    let root_path = server_cfg.normalized_root_path();
    let full_non_application_root_path = server_cfg.full_non_application_root_path();
    let request_body_limit = server_cfg.request_body_limit;

    let mut cx = H::create_context(config).await;

    let mut route_table = HashMap::with_capacity(128);
    let mut paths = BTreeMap::new();
    let mut components = Components::default();

    cx.resolve_by_type_async::<Arc<dyn Controller>>()
        .await
        .into_iter()
        .for_each(|c| {
            c.insert_routes(&mut cx, &mut route_table, &mut paths, &mut components);
        });

    let info = H::openapi_info();

    let paths = paths
        .into_iter()
        .map(|(k, v)| (root_path.clone().join(k).into(), v))
        .collect();

    let api = OpenAPI {
        openapi: "3.0.0".to_string(),
        info,
        paths: Paths {
            paths,
            ..Default::default()
        },
        components: Some(components),
        ..Default::default()
    };

    cx.insert_singleton(api);

    let mut router = Router::default();

    for (path, map) in route_table {
        let path = root_path.clone().join(path);
        router.insert(path, MethodRouter::from(map)).unwrap();
    }

    for p in cx.resolve_by_type_async::<Arc<dyn Plugin>>().await {
        let (path, map) = p.create_route(&mut cx);

        let path = full_non_application_root_path.clone().join(path);

        tracing::info!("register plugin: {}", path);

        router.insert(path, MethodRouter::from(map)).unwrap();
    }

    H::after_routes(&router);

    let router = H::add_middlewares(router, &mut cx).await;

    let router = router.before(move |mut req| async move {
        req.head
            .extensions
            .insert(RequestBodyLimit(request_body_limit));

        Ok(req)
    });

    H::start_server(router, &mut cx).await.unwrap();
}

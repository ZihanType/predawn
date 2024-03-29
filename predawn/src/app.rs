use std::{collections::BTreeMap, io, net::SocketAddr, sync::Arc};

use config::ConfigError;
use predawn_core::{
    openapi::{self, Components, Info, OpenAPI, Paths},
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
    fn load_config(env: &Environment) -> Result<Config, ConfigError> {
        Config::load(env)
    }

    fn init_logger(config: &Config) {
        let cfg = LoggerConfig::from(config);

        tracing_subscriber::fmt()
            .with_max_level(tracing::Level::from(cfg.level))
            .init();
    }

    #[allow(async_fn_in_trait)]
    async fn create_context(config: Config, env: Environment) -> Context {
        Context::options()
            .singleton(config)
            .singleton(env)
            .auto_register_async()
            .await
    }

    fn openapi_info(cx: &mut Context) -> Info {
        let _cx = cx;
        Default::default()
    }

    fn openapi_servers(cx: &mut Context) -> Vec<openapi::Server> {
        let _cx = cx;
        Default::default()
    }

    fn after_routes(router: &Router) {
        let _router = router;
    }

    #[allow(async_fn_in_trait)]
    async fn before_run<H: Handler>(cx: Context, router: H) -> (Context, impl Handler) {
        (cx, router)
    }

    #[allow(async_fn_in_trait)]
    async fn start_server<H: Handler>(cx: &mut Context, router: H) -> io::Result<()> {
        cx.just_create_single::<ServerConfig>();
        let cfg = cx.get_single::<ServerConfig>();

        let socket_addr = SocketAddr::new(cfg.ip, cfg.port);

        let listener = TcpListener::bind(socket_addr).await?;

        Server::new(listener)
            .run_with_graceful_shutdown(router, async {
                let _ = signal::ctrl_c().await;
            })
            .await
    }
}

pub async fn run_app<H: Hooks>() {
    let env = Environment::resolve_from_env();

    let (mut cx, router) = create_app::<H>(env).await;

    H::start_server(&mut cx, router).await.unwrap();
}

pub async fn create_app<H: Hooks>(env: Environment) -> (Context, impl Handler) {
    let config = H::load_config(&env).unwrap();

    H::init_logger(&config);

    let config = H::load_config(&env).unwrap();

    let server_cfg = ServerConfig::from(&config);
    let request_body_limit = server_cfg.request_body_limit;
    let root_path = server_cfg.root_path.clone();
    let full_non_application_root_path = server_cfg.full_non_application_root_path();

    let mut cx = H::create_context(config, env).await;

    let mut route_table = BTreeMap::new();
    let mut paths = BTreeMap::new();
    let mut components = Components::default();

    cx.resolve_by_type_async::<Arc<dyn Controller>>()
        .await
        .into_iter()
        .for_each(|c| {
            c.insert_routes(&mut cx, &mut route_table, &mut paths, &mut components);
        });

    let info = H::openapi_info(&mut cx);
    let servers = H::openapi_servers(&mut cx);

    let paths = paths
        .into_iter()
        .map(|(k, v)| (root_path.clone().join(k).into(), v))
        .collect();

    let api = OpenAPI {
        openapi: "3.0.0".to_string(),
        info,
        servers,
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

        let err_msg = format!("failed to insert route `{}`", path);

        router
            .insert(path, MethodRouter::from(map))
            .unwrap_or_else(|e| panic!("{}: {:?}", err_msg, e));
    }

    for p in cx.resolve_by_type_async::<Arc<dyn Plugin>>().await {
        let (path, map) = p.create_route(&mut cx);

        let path = full_non_application_root_path.clone().join(path);

        tracing::info!("registering plugin: {}", path);

        let err_msg = format!("failed to insert route `{}`", path);

        router
            .insert(path, MethodRouter::from(map))
            .unwrap_or_else(|e| panic!("{}: {:?}", err_msg, e));
    }

    H::after_routes(&router);

    let (cx, router) = H::before_run(cx, router).await;

    let router = router.before(move |mut req| async move {
        req.head
            .extensions
            .insert(RequestBodyLimit(request_body_limit));

        Ok(req)
    });

    (cx, router)
}

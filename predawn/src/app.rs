use core::panic;
use std::{
    collections::{BTreeMap, HashMap},
    io,
    net::SocketAddr,
    sync::Arc,
};

use config::ConfigError;
use http::Method;
use indexmap::IndexMap;
use predawn_core::{
    openapi::{self, Components, Info, OpenAPI, PathItem, Paths, ReferenceOr, SecurityRequirement},
    request::BodyLimit,
};
use rudi::Context;
use tokio::net::TcpListener;

use crate::{
    any_map::AnyMap,
    config::{Config, logger::LoggerConfig, server::ServerConfig},
    controller::Controller,
    environment::Environment,
    handler::{Handler, HandlerExt},
    plugin::Plugin,
    route::{MethodRouter, Router},
    server::{Server, shutdown_signal},
};

pub trait Hooks {
    fn load_config(env: &Environment) -> Result<Config, ConfigError> {
        Config::load(env)
    }

    fn init_logger(config: &Config, map: &mut AnyMap) {
        let _map = map;

        let cfg = LoggerConfig::new(config);

        if let Some(level) = cfg.level.as_tracing_level() {
            tracing_subscriber::fmt().with_max_level(level).init();
        }
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

    fn openapi_security_requirements(cx: &mut Context) -> Vec<SecurityRequirement> {
        let _cx = cx;
        Default::default()
    }

    fn openapi_security_schemes(cx: &mut Context) -> BTreeMap<String, openapi::SecurityScheme> {
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
            .run_with_graceful_shutdown(router, shutdown_signal())
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

    let mut map = AnyMap::default();

    H::init_logger(&config, &mut map);

    let config = H::load_config(&env).unwrap();

    let server_cfg = ServerConfig::new(&config);
    let request_body_limit = server_cfg.request_body_limit;
    let root_path = server_cfg.root_path.clone();
    let full_non_application_root_path = server_cfg.full_non_application_root_path();

    let mut cx = H::create_context(config, env).await;
    cx.insert_single_owner(map);

    let mut route_table = IndexMap::with_capacity(128);
    let mut paths = IndexMap::with_capacity(128);
    let mut schemas = BTreeMap::new();
    let mut schemas_in_progress = Vec::with_capacity(16);
    let mut security_schemes = BTreeMap::new();
    let mut tags = BTreeMap::new();

    cx.resolve_by_type_async::<Arc<dyn Controller>>()
        .await
        .into_iter()
        .for_each(|c| {
            c.insert_routes(
                &mut cx,
                &mut route_table,
                &mut paths,
                &mut schemas,
                &mut schemas_in_progress,
                &mut security_schemes,
                &mut tags,
            );
        });

    let info = H::openapi_info(&mut cx);
    let servers = H::openapi_servers(&mut cx);
    let security = H::openapi_security_requirements(&mut cx);

    let schemas = schemas
        .into_iter()
        .map(|(name, schema)| (name, ReferenceOr::Item(schema)))
        .collect();

    let mut duplicate_endpoints: HashMap<String, Vec<Method>> = HashMap::new();
    let mut appeared_method_cache: Vec<Method> = Vec::new();

    let paths = paths
        .into_iter()
        .map(|(path, operations)| {
            let path = root_path.clone().join(path).into_inner();

            let mut path_item = PathItem::default();

            appeared_method_cache.clear();

            operations.into_iter().for_each(|(method, operation)| {
                match method {
                    Method::GET => path_item.get = Some(operation),
                    Method::POST => path_item.post = Some(operation),
                    Method::PUT => path_item.put = Some(operation),
                    Method::DELETE => path_item.delete = Some(operation),
                    Method::HEAD => path_item.head = Some(operation),
                    Method::OPTIONS => path_item.options = Some(operation),
                    Method::PATCH => path_item.patch = Some(operation),
                    Method::TRACE => path_item.trace = Some(operation),
                    _ => {
                        tracing::info!(
                            "the `{method} {path}` endpoint does not appear in the OpenAPI documentation"
                        )
                    }
                }

                if !appeared_method_cache.contains(&method) {
                    appeared_method_cache.push(method);
                } else if !duplicate_endpoints.contains_key(&path) {
                    duplicate_endpoints.insert(path.clone(), vec![method]);
                } else {
                    let duplicate_methods = duplicate_endpoints.get_mut(&path).unwrap();
                    if !duplicate_methods.contains(&method) {
                        duplicate_methods.push(method);
                    }
                }
            });

            (path, ReferenceOr::Item(path_item))
        })
        .collect();

    if !duplicate_endpoints.is_empty() {
        panic!("duplicate endpoints: {:#?}", duplicate_endpoints);
    }

    let mut tag_name_to_type_names: BTreeMap<_, Vec<_>> = BTreeMap::new();

    let tags = tags
        .into_iter()
        .map(|(tag_type_name, (tag_name, tag))| {
            debug_assert_eq!(tag_name, tag.name);

            tag_name_to_type_names
                .entry(tag_name)
                .or_default()
                .push(tag_type_name);

            tag
        })
        .collect::<Vec<_>>();

    // retains only the tag types with the same tag name
    tag_name_to_type_names.retain(|_, v| v.len() > 1);

    // if tag_name_to_type_names is not empty, it should panic
    // because it means that there are multiple tag types with the same tag name
    if !tag_name_to_type_names.is_empty() {
        panic!(
            "multiple tags with the same name: {:#?}",
            tag_name_to_type_names
        );
    }

    let mut schemes_name_to_type_names: BTreeMap<_, Vec<_>> = BTreeMap::new();

    let mut security_schemes = security_schemes
        .into_iter()
        .map(|(scheme_type_name, (scheme_name, scheme))| {
            schemes_name_to_type_names
                .entry(scheme_name)
                .or_default()
                .push(scheme_type_name);

            (scheme_name.to_string(), ReferenceOr::Item(scheme))
        })
        .collect::<IndexMap<_, _>>();

    // retains only the security scheme types with the same scheme name
    schemes_name_to_type_names.retain(|_, v| v.len() > 1);

    // if schemes_name_to_type_names is not empty, it should panic
    // because it means that there are multiple security scheme types with the same scheme name
    if !schemes_name_to_type_names.is_empty() {
        panic!(
            "multiple security scheme types with the same name: {:#?}",
            schemes_name_to_type_names
        );
    }

    let mut duplicate_schemes = BTreeMap::new();

    // `openapi_security_schemes` means the global security schemes
    // `security_schemes` means all the security schemes
    // if there are multiple security schemes with the same name, it should panic
    H::openapi_security_schemes(&mut cx)
        .into_iter()
        .for_each(|(name, scheme)| match security_schemes.get(&name) {
            Some(ReferenceOr::Item(exist_scheme)) => {
                if exist_scheme != &scheme {
                    duplicate_schemes.insert(name, (scheme, exist_scheme.clone()));
                }
            }
            Some(ReferenceOr::Reference { .. }) => unreachable!(),
            None => {
                security_schemes.insert(name, ReferenceOr::Item(scheme));
            }
        });

    if !duplicate_schemes.is_empty() {
        panic!(
            "multiple security schemes with the same name: {:#?}",
            duplicate_schemes
        );
    }

    let components = Components {
        schemas,
        responses: Default::default(),
        parameters: Default::default(),
        examples: Default::default(),
        request_bodies: Default::default(),
        headers: Default::default(),
        security_schemes,
        links: Default::default(),
        callbacks: Default::default(),
        extensions: Default::default(),
    };

    let api = OpenAPI {
        openapi: "3.0.0".to_string(),
        info,
        servers,
        paths: Paths {
            paths,
            extensions: Default::default(),
        },
        components: Some(components),
        security: Some(security),
        tags,
        external_docs: Default::default(),
        extensions: Default::default(),
    };

    cx.insert_singleton(api);

    let mut router = Router::default();
    let mut insert_errors = Vec::new();

    for (path, handlers) in route_table {
        let path = root_path.clone().join(path);

        // already checked for duplicates at `paths` above, so no need to check again here.
        let map = handlers.into_iter().collect::<IndexMap<_, _>>();

        let path = path.into_inner();
        let path_cloned = path.clone();

        if let Err(e) = router.insert(path, MethodRouter::from(map)) {
            insert_errors.push((e, path_cloned));
        }
    }

    for plugin in cx.resolve_by_type_async::<Arc<dyn Plugin>>().await {
        let (path, map) = plugin.create_route(&mut cx);

        let path = full_non_application_root_path.clone().join(path);

        tracing::info!("registering plugin: {}", path);

        let path = path.into_inner();
        let path_cloned = path.clone();

        if let Err(e) = router.insert(path, MethodRouter::from(map)) {
            insert_errors.push((e, path_cloned));
        }
    }

    if !insert_errors.is_empty() {
        panic!("failed to insert paths: {:#?}", insert_errors);
    }

    H::after_routes(&router);

    let (cx, router) = H::before_run(cx, router).await;

    let router = router.before(move |mut req| async move {
        *req.body_limit() = BodyLimit(request_body_limit);
        Ok(req)
    });

    (cx, router)
}

use std::{
    collections::{BTreeMap, HashSet},
    time::Duration,
};

use http::StatusCode;
use predawn::{
    any_map::AnyMap,
    app::{run_app, Hooks},
    config::{logger::LoggerConfig, Config},
    controller,
    extract::{
        multipart::{JsonField, Multipart, Upload},
        Path, Query,
    },
    handler::{Handler, HandlerExt},
    middleware::{TowerLayerCompatExt, Tracing},
    openapi::{self, SecurityRequirement},
    payload::{Form, Json},
    response::Download,
    response_error::ResponseError,
    route::Router,
    SecurityScheme, Tag, ToParameters, ToSchema,
};
use rudi::{Context, Singleton};
use serde::{Deserialize, Serialize};
use tower::limit::RateLimitLayer;
use tower_http::compression::CompressionLayer;
use tracing_subscriber::{
    fmt::writer::MakeWriterExt, layer::SubscriberExt, util::SubscriberInitExt,
};

struct App;

impl Hooks for App {
    fn init_logger(config: &Config, map: &mut AnyMap) {
        let cfg = LoggerConfig::new(config);

        let file_appender = tracing_appender::rolling::daily(
            concat!(env!("CARGO_MANIFEST_DIR"), "/log"),
            "hello-world.log",
        );
        let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

        let file_layer = tracing_subscriber::fmt::layer()
            .with_ansi(false)
            .with_writer(non_blocking.with_max_level(tracing::Level::from(cfg.level)));

        let stdout_layer = tracing_subscriber::fmt::layer()
            .with_writer(std::io::stdout.with_max_level(tracing::Level::from(cfg.level)));

        tracing_subscriber::registry()
            .with(file_layer)
            .with(stdout_layer)
            .init();

        map.insert(guard);
    }

    async fn before_run<H: Handler>(mut cx: Context, router: H) -> (Context, impl Handler) {
        let t = cx.resolve::<Tracing>();

        let router = router
            .with(CompressionLayer::new().zstd(true).compat())
            .with(t);

        (cx, router)
    }

    // set global security requirements
    fn openapi_security_requirements(_: &mut Context) -> Vec<SecurityRequirement> {
        let mut map = SecurityRequirement::default();
        map.insert(MyScheme1::NAME.to_string(), Vec::new());

        vec![map]
    }

    // set global security schemes
    fn openapi_security_schemes(_: &mut Context) -> BTreeMap<String, openapi::SecurityScheme> {
        let mut map = BTreeMap::new();
        map.insert(MyScheme1::NAME.to_string(), MyScheme1::create());
        map
    }

    fn after_routes(router: &Router) {
        router.routes().iter().for_each(|(route, methods)| {
            println!("{}: {:?}", route, methods);
        });
    }
}

#[tokio::main]
async fn main() {
    run_app::<App>().await
}

#[derive(Clone)]
#[Singleton]
pub struct MyController {}

/// This is a controller.
#[derive(Tag)]
#[tag(rename = "Controller Tag")]
struct Controller;

/// Hello
#[derive(Tag)]
struct Hello;

#[derive(SecurityScheme)]
#[http(scheme = bearer)]
struct MyScheme1;

#[derive(SecurityScheme)]
#[http(scheme = basic)]
struct MyScheme2;

#[controller(tags = [Controller])]
impl MyController {
    /// no argument, no return
    ///
    /// # Example
    ///
    /// ```shell
    /// curl http://localhost:9612/no_arg
    /// ```
    #[handler(paths = ["/no_arg"], methods = [GET], security = [{}, { MyScheme2: [] }])] // override the global security
    async fn no_arg(&self) {}

    #[handler(methods = [GET, POST, PUT], middleware = add_middlewares, tags = [Hello])]
    async fn hello(&self, name: String) -> Result<String, MyError> {
        Ok(format!("hello, {}", name))
    }

    #[handler(paths = ["/json"], methods = [POST], security = [{ MyScheme2: ["read", "write"] }])]
    async fn json_person(&self, mut person: Json<Person>) -> Json<Person> {
        person.age += 1;
        person
    }

    #[handler(paths = ["/form"], methods = [POST, GET])]
    async fn form_person(&self, mut person: Form<Person>) -> Form<Person> {
        person.age += 1;
        person
    }

    #[handler(paths = ["/form_multi_value"], methods = [POST, GET])]
    async fn form_multi_value(&self, Form(mut values): Form<MultiValue>) -> Form<MultiValue> {
        values.values.push(1);
        Form(values)
    }

    #[handler(paths = ["/query"], methods = [POST, GET])]
    async fn query_person(&self, Query(multi_value): Query<MultiValue>) -> Json<MultiValue> {
        Json(multi_value)
    }

    #[handler(paths = ["/{name}/{age}"], methods = [GET])]
    async fn path_person(&self, Path(person): Path<Person>) -> Json<Person> {
        Json(person)
    }

    #[handler(paths = ["/multipart"], methods = [POST])]
    async fn multipart_person(&self, m: MultipartStruct) -> Json<Person> {
        let MultipartStruct {
            person: JsonField(person),
            sex,
            files,
        } = m;

        println!("sex: {sex}");

        for file in files {
            println!(
                "file name: {}, content type: {}, file size: {}",
                file.file_name(),
                file.content_type(),
                file.bytes().len(),
            );
        }

        Json(person)
    }

    #[handler(paths = ["/download_from_memory"], methods = [GET])]
    async fn download_from_memory(&self) -> Download<Json<Person>> {
        let json = Json(Person {
            name: Some("Alice".into()),
            age: 18,
        });

        Download::attachment(json, "test.json")
    }

    #[handler(paths = ["/download_from_disk"], methods = [GET])]
    async fn download_from_disk(&self) -> Download<Vec<u8>> {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("test.json");

        let bytes = std::fs::read(path).unwrap();

        Download::attachment(bytes, "test.json")
    }

    #[handler(paths = ["/unit_enum"], methods = [GET])]
    async fn unit_enum(&self) -> Json<UnitEnum> {
        Json(UnitEnum::A)
    }

    #[handler(paths = ["/unit_with_description"], methods = [GET])]
    async fn unit_with_description(&self) -> Json<UnitWithDescription> {
        Json(UnitWithDescription::A)
    }

    #[handler(paths = ["/complex_enum"], methods = [GET])]
    async fn complex_enum(&self) -> Json<ComplexEnum> {
        Json(ComplexEnum::A)
    }

    #[handler(paths = ["/nested_schema"], methods = [GET])]
    async fn nested_schema(&self) -> Json<Nested> {
        Json(Nested {
            name: "Hello".to_string(),
            inner: None,
        })
    }
}

fn add_middlewares<H: Handler>(_: &mut Context, handler: H) -> impl Handler {
    handler
        .before(|req| async {
            println!("before: {:?}", req);
            Ok(req)
        })
        .with(RateLimitLayer::new(1, Duration::from_secs(3)).compat())
}

#[Singleton]
fn CreateMiddleware() -> Tracing {
    Tracing
}

/// A person.
#[derive(Debug, Serialize, Deserialize, ToSchema, ToParameters, Multipart)]
struct Person {
    /// The name of the person.
    name: Option<String>,
    /// The age of the person.
    age: u16,
}

#[derive(Serialize, Deserialize, ToSchema, ToParameters)]
struct MultiValue {
    /// value
    #[serde(rename = "value")]
    values: Vec<u16>,
}

#[derive(Debug, ToSchema, Multipart)]
struct MultipartStruct {
    person: JsonField<Person>,
    sex: String,
    files: [Upload; 2],
}

#[allow(dead_code)]
#[derive(Debug, ToSchema, Serialize)]
enum UnitEnum {
    A,
    B,
}

#[allow(dead_code)]
#[derive(Debug, ToSchema, Serialize)]
enum UnitWithDescription {
    /// Hello
    A,
    /// World
    B,
}

#[allow(dead_code)]
#[derive(Debug, ToSchema, Serialize)]
enum ComplexEnum {
    /// Hello
    A,
    B(i32),
    C {
        name: String,
        age: u16,
    },
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
struct Nested {
    name: String,
    inner: Option<Box<Nested>>,
}

#[derive(Debug, thiserror::Error)]
#[error("my error")]
struct MyError;

impl ResponseError for MyError {
    fn as_status(&self) -> StatusCode {
        StatusCode::INTERNAL_SERVER_ERROR
    }

    fn status_codes() -> HashSet<StatusCode> {
        [StatusCode::INTERNAL_SERVER_ERROR].into()
    }
}

#[cfg(test)]
mod tests {
    use predawn::test_client::TestClient;

    use super::*;

    #[tokio::test]
    async fn test_my_controller() {
        let client = TestClient::new::<App>().await;
        let res = client.get("/no_arg").send().await.unwrap();
        assert_eq!(res.status(), 200);

        let res = client.post("/").body("world").send().await.unwrap();
        assert_eq!(res.status(), 200);
        assert_eq!(res.text().await.unwrap(), "hello, world");
    }
}

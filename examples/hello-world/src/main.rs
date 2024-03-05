use std::{collections::HashSet, time::Duration};

use http::StatusCode;
use predawn::{
    app::{run_app, Hooks},
    controller,
    extract::{path::Path, query::Query},
    handler::{Handler, HandlerExt},
    middleware::{tower_compat::TowerLayerCompatExt, tracing::Tracing},
    payload::{form::Form, json::Json},
    response_error::ResponseError,
    ToParameters, ToSchema,
};
use rudi::{Context, Singleton};
use serde::{Deserialize, Serialize};
use tower::limit::RateLimitLayer;

struct App;

impl Hooks for App {
    async fn add_middlewares<H: Handler>(router: H, cx: &mut Context) -> impl Handler {
        router.with(cx.resolve::<Tracing>())
    }

    fn after_routes(router: &predawn::route::Router) {
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

#[controller]
impl MyController {
    #[handler(paths = ["/no_arg"], methods = [GET])]
    async fn no_arg(&self) {}

    #[handler(methods = [GET, POST, PUT], middleware = add_middlewares)]
    async fn hello(&self, name: String) -> Result<String, MyError> {
        Ok(format!("hello, {}", name))
    }

    #[handler(paths = ["/json"], methods = [POST])]
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

    #[handler(paths = ["/:name/:age"], methods = [GET])]
    async fn path_person(&self, Path(person): Path<Person>) -> Json<Person> {
        Json(person)
    }
}

fn add_middlewares<H: Handler>(handler: H, _: &mut Context) -> impl Handler {
    handler
        .before(|req| async {
            println!("before: {:?}", req);
            Ok(req)
        })
        .with(RateLimitLayer::new(1, Duration::from_secs(30)).compat())
}

#[Singleton]
fn CreateMiddleware() -> Tracing {
    Tracing
}

#[derive(Serialize, Deserialize, ToSchema, ToParameters)]
struct Person {
    name: Option<String>,
    age: u16,
}

#[derive(Serialize, Deserialize, ToSchema, ToParameters)]
struct MultiValue {
    #[serde(rename = "value")]
    values: Vec<u16>,
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

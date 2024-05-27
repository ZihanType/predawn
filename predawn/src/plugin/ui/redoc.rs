use std::sync::Arc;

use http::Method;
use indexmap::IndexMap;
use rudi::{Context, Singleton};

use crate::{config::Config, handler::DynHandler, normalized_path::NormalizedPath, plugin::Plugin};

const TEMPLATE: &str = r###"
<!DOCTYPE html>
<html>
  <head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1" />
    <meta name="description" content="{{description}}" />
    <title>{{title}}</title>
    <link
      href="https://fonts.googleapis.com/css?family=Montserrat:300,400,700|Roboto:300,400,700"
      rel="stylesheet"
    />
  </head>
  <body>
    <redoc spec-url="{{spec_url}}"></redoc>
    <script src="{{js_url}}"></script>
  </body>
</html>
"###;

#[derive(Clone, Debug)]
pub struct Redoc {
    description: Box<str>,
    title: Box<str>,
    js_url: Box<str>,
    spec_url: Box<str>,
}

impl Plugin for Redoc {
    fn create_route(
        self: Arc<Self>,
        cx: &mut Context,
    ) -> (NormalizedPath, IndexMap<Method, DynHandler>) {
        super::create_route(cx, |c| c.redoc_path, self.as_html())
    }
}

fn condition(cx: &Context) -> bool {
    !cx.contains_provider::<Redoc>()
}

#[Singleton(condition = condition)]
fn RedocRegister(#[di(ref)] cfg: &Config) -> Redoc {
    let json_path = super::json_path(cfg).into_inner();
    Redoc::new(json_path)
}

#[Singleton(name = std::any::type_name::<Redoc>())]
fn RedocToPlugin(redoc: Redoc) -> Arc<dyn Plugin> {
    Arc::new(redoc)
}

impl Redoc {
    pub fn new<T>(spec_url: T) -> Self
    where
        T: Into<Box<str>>,
    {
        Self {
            description: Box::from("Redoc"),
            title: Box::from("Redoc"),
            js_url: Box::from("https://cdn.redoc.ly/redoc/latest/bundles/redoc.standalone.js"),
            spec_url: spec_url.into(),
        }
    }

    pub fn as_html(&self) -> String {
        TEMPLATE
            .replacen("{{description}}", &self.description, 1)
            .replacen("{{title}}", &self.title, 1)
            .replacen("{{js_url}}", &self.js_url, 1)
            .replacen("{{spec_url}}", &self.spec_url, 1)
    }
}

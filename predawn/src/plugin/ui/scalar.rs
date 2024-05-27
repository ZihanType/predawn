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
  </head>
  <body>
    <script id="api-reference" data-url="{{spec_url}}"></script>

    <script>
      const configuration = {
        theme: "default",
      };

      document.getElementById("api-reference").dataset.configuration =
        JSON.stringify(configuration);
    </script>
    <script src="{{js_url}}"></script>
  </body>
</html>
"###;

#[derive(Clone, Debug)]
pub struct Scalar {
    description: Box<str>,
    title: Box<str>,
    js_url: Box<str>,
    spec_url: Box<str>,
}

impl Plugin for Scalar {
    fn create_route(
        self: Arc<Self>,
        cx: &mut Context,
    ) -> (NormalizedPath, IndexMap<Method, DynHandler>) {
        super::create_route(cx, |c| c.scalar_path, self.as_html())
    }
}

fn condition(cx: &Context) -> bool {
    !cx.contains_provider::<Scalar>()
}

#[Singleton(condition = condition)]
fn ScalarRegister(#[di(ref)] cfg: &Config) -> Scalar {
    let json_path = super::json_path(cfg).into_inner();
    Scalar::new(json_path)
}

#[Singleton(name = std::any::type_name::<Scalar>())]
fn ScalarToPlugin(scalar: Scalar) -> Arc<dyn Plugin> {
    Arc::new(scalar)
}

impl Scalar {
    pub fn new<T>(spec_url: T) -> Self
    where
        T: Into<Box<str>>,
    {
        Self {
            description: Box::from("Scalar"),
            title: Box::from("Scalar"),
            js_url: Box::from("https://cdn.jsdelivr.net/npm/@scalar/api-reference"),
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

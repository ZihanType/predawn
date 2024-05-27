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
    <script type="module" src="{{js_url}}"></script>
  </head>
  <body>
    <openapi-explorer spec-url="{{spec_url}}">
    </openapi-explorer>
  </body>
</html>
"###;

#[derive(Clone, Debug)]
pub struct OpenapiExplorer {
    description: Box<str>,
    title: Box<str>,
    js_url: Box<str>,
    spec_url: Box<str>,
}

impl Plugin for OpenapiExplorer {
    fn create_route(
        self: Arc<Self>,
        cx: &mut Context,
    ) -> (NormalizedPath, IndexMap<Method, DynHandler>) {
        super::create_route(cx, |c| c.openapi_explorer_path, self.as_html())
    }
}

fn condition(cx: &Context) -> bool {
    !cx.contains_provider::<OpenapiExplorer>()
}

#[Singleton(condition = condition)]
fn OpenapiExplorerRegister(#[di(ref)] cfg: &Config) -> OpenapiExplorer {
    let json_path = super::json_path(cfg).into_inner();
    OpenapiExplorer::new(json_path)
}

#[Singleton(name = std::any::type_name::<OpenapiExplorer>())]
fn OpenapiExplorerToPlugin(openapi_explorer: OpenapiExplorer) -> Arc<dyn Plugin> {
    Arc::new(openapi_explorer)
}

impl OpenapiExplorer {
    pub fn new<T>(spec_url: T) -> Self
    where
        T: Into<Box<str>>,
    {
        Self {
            description: Box::from("Openapi Explorer"),
            title: Box::from("Openapi Explorer"),
            js_url: Box::from(
                "https://unpkg.com/openapi-explorer@0/dist/browser/openapi-explorer.min.js",
            ),
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

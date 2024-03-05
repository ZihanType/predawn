use std::{collections::HashMap, sync::Arc};

use http::Method;
use rudi::{Context, Singleton};

use crate::{config::Config, handler::Handler, normalized_path::NormalizedPath, plugin::Plugin};

const TEMPLATE: &str = r#"
<!doctype html>
<html>
  <head>
    <meta
      name="description"
      content="{{description}}"
    />
    <title>{{title}}</title>
    {{keywords}}
    <meta charset="utf-8">
    <script type="module" src="{{js_url}}"></script>
  </head>
  <body>
    <rapi-doc
      spec-url = "{{spec_url}}"
    >
    </rapi-doc>
  </body>
</html>
"#;

#[derive(Clone, Debug)]
pub struct RapiDoc {
    description: String,
    title: String,
    keywords: Option<String>,
    js_url: String,
    spec_url: String,
}

impl Plugin for RapiDoc {
    fn create_route(
        self: Arc<Self>,
        cx: &mut Context,
    ) -> (NormalizedPath, HashMap<Method, Arc<dyn Handler>>) {
        super::create_route(cx, |c| c.normalized_rapidoc_path(), self.as_html())
    }
}

fn condition(cx: &Context) -> bool {
    cx.get_provider::<RapiDoc>().is_none()
}

#[Singleton(condition = condition)]
fn RapiDocRegister(#[di(ref)] cfg: &Config) -> RapiDoc {
    let json_path = super::json_path(cfg).into_inner();
    RapiDoc::new(json_path)
}

#[Singleton(name = std::any::type_name::<RapiDoc>())]
fn RapiDocToPlugin(rapidoc: RapiDoc) -> Arc<dyn Plugin> {
    Arc::new(rapidoc)
}

impl RapiDoc {
    pub fn new<T>(spec_url: T) -> Self
    where
        T: Into<String>,
    {
        Self {
            description: String::from("RapiDoc"),
            title: String::from("RapiDoc"),
            keywords: None,
            js_url: String::from("https://unpkg.com/rapidoc/dist/rapidoc-min.js"),
            spec_url: spec_url.into(),
        }
    }

    pub fn as_html(&self) -> String {
        let keywords = self.keywords.as_ref().map_or(String::new(), |keywords| {
            format!(
                "<meta name=\"keywords\" content=\"{}\">",
                keywords
                    .split(',')
                    .map(|s| s.trim())
                    .collect::<Vec<_>>()
                    .join(",")
            )
        });

        TEMPLATE
            .replacen("{{description}}", &self.description, 1)
            .replacen("{{title}}", &self.title, 1)
            .replacen("{{keywords}}", &keywords, 1)
            .replacen("{{js_url}}", &self.js_url, 1)
            .replacen("{{spec_url}}", &self.spec_url, 1)
    }
}

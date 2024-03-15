use std::{collections::HashMap, sync::Arc};

use http::Method;
use rudi::{Context, Singleton};

use crate::{config::Config, handler::Handler, normalized_path::NormalizedPath, plugin::Plugin};

const TEMPLATE: &str = r#"
<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <meta
    name="description"
    content="{{description}}"
  />
  <title>{{title}}</title>
  <link rel="stylesheet" href="{{css_url}}" />
</head>
<body>
<div id="swagger-ui"></div>
<script src="{{bundle_js_url}}" crossorigin></script>
<script src="{{standalone_preset_url}}" crossorigin></script>
<script>
  window.onload = () => {
    window.ui = SwaggerUIBundle({
      url: '{{spec_url}}',
      dom_id: '#swagger-ui',
      presets: [
        SwaggerUIBundle.presets.apis,
        SwaggerUIStandalonePreset
      ],
      layout: "StandaloneLayout",
    });
  };
</script>
</body>
</html>
"#;

#[derive(Debug, Clone)]
pub struct SwaggerUI {
    description: String,
    title: String,
    css_url: String,
    bundle_js_url: String,
    standalone_preset_url: String,
    spec_url: String,
}

impl Plugin for SwaggerUI {
    fn create_route(
        self: Arc<Self>,
        cx: &mut Context,
    ) -> (NormalizedPath, HashMap<Method, Arc<dyn Handler>>) {
        super::create_route(cx, |c| c.swagger_ui_path, self.as_html())
    }
}

fn condition(cx: &Context) -> bool {
    cx.get_provider::<SwaggerUI>().is_none()
}

#[Singleton(condition = condition)]
fn SwaggerUIRegister(#[di(ref)] cfg: &Config) -> SwaggerUI {
    let json_path = super::json_path(cfg).into_inner();
    SwaggerUI::new(json_path)
}

#[Singleton(name = std::any::type_name::<SwaggerUI>())]
fn SwaggerUIToPlugin(ui: SwaggerUI) -> Arc<dyn Plugin> {
    Arc::new(ui)
}

impl SwaggerUI {
    pub fn new<T>(spec_url: T) -> Self
    where
        T: Into<String>,
    {
        Self {
            description: String::from("SwaggerUI"),
            title: String::from("SwaggerUI"),
            css_url: String::from("https://unpkg.com/swagger-ui-dist/swagger-ui.css"),
            bundle_js_url: String::from("https://unpkg.com/swagger-ui-dist/swagger-ui-bundle.js"),
            standalone_preset_url: String::from(
                "https://unpkg.com/swagger-ui-dist/swagger-ui-standalone-preset.js",
            ),
            spec_url: spec_url.into(),
        }
    }

    pub fn description<T>(mut self, description: T) -> Self
    where
        T: Into<String>,
    {
        self.description = description.into();
        self
    }

    pub fn title<T>(mut self, title: T) -> Self
    where
        T: Into<String>,
    {
        self.title = title.into();
        self
    }

    pub fn css_url<T>(mut self, css_url: T) -> Self
    where
        T: Into<String>,
    {
        self.css_url = css_url.into();
        self
    }

    pub fn bundle_js_url<T>(mut self, bundle_js_url: T) -> Self
    where
        T: Into<String>,
    {
        self.bundle_js_url = bundle_js_url.into();
        self
    }

    pub fn standalone_preset_url<T>(mut self, standalone_preset_url: T) -> Self
    where
        T: Into<String>,
    {
        self.standalone_preset_url = standalone_preset_url.into();
        self
    }

    pub fn as_html(&self) -> String {
        TEMPLATE
            .replacen("{{description}}", &self.description, 1)
            .replacen("{{title}}", &self.title, 1)
            .replacen("{{css_url}}", &self.css_url, 1)
            .replacen("{{bundle_js_url}}", &self.bundle_js_url, 1)
            .replacen("{{standalone_preset_url}}", &self.standalone_preset_url, 1)
            .replacen("{{spec_url}}", &self.spec_url, 1)
    }
}

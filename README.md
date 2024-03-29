# Predawn

[![Crates.io version](https://img.shields.io/crates/v/predawn.svg?style=flat-square)](https://crates.io/crates/predawn)
[![docs.rs docs](https://img.shields.io/badge/docs-latest-blue.svg?style=flat-square)](https://docs.rs/predawn)

`predawn` is a Rust web framework like `Spring Boot`.

```rust
use predawn::{
    app::{run_app, Hooks},
    controller,
};
use rudi::Singleton;

struct App;

impl Hooks for App {}

#[tokio::main]
async fn main() {
    run_app::<App>().await;
}

#[derive(Clone)]
#[Singleton]
struct Controller;

#[controller]
impl Controller {
    #[handler(paths = ["/"], methods = [post])]
    async fn hello(&self, name: String) -> String {
        format!("Hello {name}")
    }
}
```

## Features

- Built-in OpenAPI support.
- Automatic dependency injection.
- Programmable configuration.

More examples can be found in the [examples](./examples/) directories.

## More complex example

```rust
use std::sync::Arc;

use async_trait::async_trait;
use predawn::{
    app::{run_app, Hooks},
    controller,
};
use rudi::Singleton;

struct App;

impl Hooks for App {}

#[tokio::main]
async fn main() {
    run_app::<App>().await;
}

#[async_trait]
trait Service: Send + Sync {
    fn arc(self) -> Arc<dyn Service>
    where
        Self: Sized + 'static,
    {
        Arc::new(self)
    }

    async fn hello(&self) -> String;
}

#[derive(Clone)]
#[Singleton(binds = [Service::arc])]
struct ServiceImpl;

#[async_trait]
impl Service for ServiceImpl {
    async fn hello(&self) -> String {
        "Hello, World!".to_string()
    }
}

#[derive(Clone)]
#[Singleton]
struct Controller {
    svc: Arc<dyn Service>,
}

#[controller]
impl Controller {
    #[handler(paths = ["/"], methods = [GET])]
    async fn hello(&self) -> String {
        self.svc.hello().await
    }
}
```

## Credits

- [axum](https://github.com/tokio-rs/axum)
- [poem](https://github.com/poem-web/poem)
- [loco](https://github.com/loco-rs/loco)
- [volo-http](https://github.com/cloudwego/volo)
- [salvo](https://github.com/salvo-rs/salvo)

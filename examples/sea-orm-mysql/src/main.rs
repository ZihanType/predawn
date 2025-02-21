mod entity;

use predawn::{
    ToParameters,
    app::{Hooks, run_app},
    controller,
    extract::Query,
    handler::{Handler, HandlerExt},
    middleware::Tracing,
};
use predawn_sea_orm::SeaOrmMiddleware;
use rudi::{Context, Singleton};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use serde::Deserialize;

struct App;

impl Hooks for App {
    async fn before_run<H: Handler>(mut cx: Context, router: H) -> (Context, impl Handler) {
        let db = cx.resolve_async::<SeaOrmMiddleware>().await;

        let router = router.with(db).with(Tracing);

        (cx, router)
    }
}

#[tokio::main]
async fn main() {
    run_app::<App>().await;
}

#[derive(Clone)]
#[Singleton]
pub struct MyController {}

#[controller]
impl MyController {
    #[endpoint(paths = ["/hello"], methods = [GET])]
    async fn hello(&self, Query(user): Query<User>) -> String {
        let User { name } = user;

        // create a transaction from `default` data source
        let txn = predawn_sea_orm::default_txn().await.unwrap();

        // create a transaction from `db1` data source
        // let txn = predawn_sea_orm::current_txn("db1").await.unwrap();

        // create a transaction from `db2` data source
        // let txn = predawn_sea_orm::current_txn("db2").await.unwrap();

        match entity::Entity::find()
            .filter(entity::Column::Name.eq(&name))
            .one(&txn)
            .await
            .unwrap()
        {
            Some(user) => {
                format!("Hello, {}!", user.name)
            }
            None => {
                format!("User not found: {}", name)
            }
        }
    }
}

#[derive(Debug, ToParameters, Deserialize)]
pub struct User {
    pub name: String,
}

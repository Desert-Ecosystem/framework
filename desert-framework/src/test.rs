use std::sync::Arc;

use extended_rust::string;

use crate::{
    dependency::{dep, Deps},
    manager::DependencyManager,
    service::Service,
};
use crate::{controller, get, impl_routes, post};

pub struct TestService1 {}

impl Service for TestService1 {
    async fn new(_manager: Arc<DependencyManager>) -> Self
    where
        Self: Sized,
    {
        Self {}
    }

    fn name() -> String {
        string!("1")
    }
}

impl TestService1 {
    pub async fn get_hello_from_1(&self) -> String {
        string!("hello from service 1")
    }
}

pub struct TestService2 {}

impl Service for TestService2 {
    fn deps() -> Deps {
        vec![dep::<TestService1>()]
    }

    async fn new(manager: Arc<DependencyManager>) -> Self
    where
        Self: Sized,
    {
        let service1 = manager.get::<TestService1>(&Self::name()).await.unwrap();

        println!("{}", service1.get_hello_from_1().await);
        Self {}
    }

    fn name() -> String {
        string!("2")
    }
}

pub async fn test() {
    let manager = DependencyManager::new();

    let _service1 = manager.register::<TestService1>().await;
    let _service2 = manager.register::<TestService2>().await;
}

#[controller(path = "/api")]
pub struct AppState {}

impl AppState {
    #[get("/hello")]
    pub async fn hello(&self) -> &'static str {
        "hello world"
    }

    #[get("/items/{id}")]
    pub async fn get_item(
        &self,
        axum::extract::Path(id): axum::extract::Path<String>,
    ) -> String {
        format!("item: {}", id)
    }

    #[post("/items")]
    pub async fn add_item(
        &self,
        axum::Json(body): axum::Json<String>,
    ) -> String {
        format!("added: {}", body)
    }
}

impl_routes!(AppState, [hello, get_item, add_item]);

pub async fn test_controller() {
    let controller = AppState {};

    let _router: axum::Router = controller.get_router();
}

pub async fn test_merge_controllers() {
    let c1 = AppState {};
    let router = axum::Router::new()
        .merge(c1.get_router());

    let _: axum::Router = router;
}

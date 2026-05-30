#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;

    use crate::dependency::{dep, Deps};
    use crate::manager::DependencyManager;
    use crate::service::Service;
    use crate::{controller, get, impl_routes, post};

    struct TestService1;

    impl Service for TestService1 {
        fn name() -> String {
            "TestService1".into()
        }

        async fn new(_manager: Arc<DependencyManager>) -> Self {
            Self
        }
    }

    impl TestService1 {
        fn get_hello(&self) -> &str {
            "hello from service 1"
        }
    }

    struct TestService2;

    impl Service for TestService2 {
        fn name() -> String {
            "TestService2".into()
        }

        fn deps() -> Deps {
            vec![dep::<TestService1>()]
        }

        async fn new(manager: Arc<DependencyManager>) -> Self {
            let _s1 = manager.get::<TestService1>("TestService2").await.unwrap();
            Self
        }
    }

    struct NoDepsService;

    impl Service for NoDepsService {
        fn name() -> String {
            "NoDepsService".into()
        }

        async fn new(_manager: Arc<DependencyManager>) -> Self {
            Self
        }
    }

    // === Dependency Manager Tests ===

    #[tokio::test]
    async fn register_service_without_deps() {
        let manager = DependencyManager::new();
        let _svc = manager.register::<NoDepsService>().await;
    }

    #[tokio::test]
    async fn register_service_with_deps() {
        let manager = DependencyManager::new();
        let _s1 = manager.register::<TestService1>().await;
        let _s2 = manager.register::<TestService2>().await;
    }

    #[tokio::test]
    async fn get_registered_service() {
        let manager = DependencyManager::new();
        let _s1 = manager.register::<TestService1>().await;
        let result = manager.get::<TestService1>("test").await;
        assert!(result.is_some());
    }

    #[tokio::test]
    async fn get_unregistered_service_returns_none() {
        let manager = DependencyManager::new();
        let result = manager.get::<TestService1>("test").await;
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn service_returns_correct_data() {
        let manager = DependencyManager::new();
        let s1 = manager.register::<TestService1>().await;
        assert_eq!(s1.get_hello(), "hello from service 1");
    }

    // === Controller Tests ===

    #[controller(path = "/api")]
    struct TestController;

    impl TestController {
        #[get("/hello")]
        async fn hello(&self) -> &'static str {
            "hello world"
        }

        #[get("/items/{id}")]
        async fn get_item(&self, axum::extract::Path(id): axum::extract::Path<String>) -> String {
            format!("item: {}", id)
        }

        #[post("/items")]
        async fn add_item(&self, axum::extract::Json(body): axum::extract::Json<String>) -> String {
            format!("added: {}", body)
        }
    }

    impl_routes!(TestController, [hello, get_item, add_item]);

    #[tokio::test]
    async fn controller_get_hello() {
        let controller = TestController;
        let app = controller.get_router();

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/hello")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        assert_eq!(body, "hello world");
    }

    #[tokio::test]
    async fn controller_get_item() {
        let controller = TestController;
        let app = controller.get_router();

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/items/42")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        assert_eq!(body, "item: 42");
    }

    #[tokio::test]
    async fn controller_post_item() {
        let controller = TestController;
        let app = controller.get_router();

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/items")
                    .header("content-type", "application/json")
                    .body(Body::from(r#""test item""#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        assert_eq!(body, "added: test item");
    }

    #[tokio::test]
    async fn controller_404_on_unknown_route() {
        let controller = TestController;
        let app = controller.get_router();

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/unknown")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[controller(path = "/api/other")]
    struct AnotherController;

    impl AnotherController {
        #[get("/test")]
        async fn test(&self) -> &'static str {
            "from another"
        }
    }

    impl_routes!(AnotherController, [test]);

    #[tokio::test]
    async fn merge_different_controllers() {
        let c1 = TestController;
        let c2 = AnotherController;

        let app = axum::Router::new()
            .merge(c1.get_router())
            .merge(c2.get_router());

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/hello")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/other/test")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        assert_eq!(body, "from another");
    }
}

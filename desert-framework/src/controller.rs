pub trait Controller {
    type State: Send + Sync + 'static;
    fn register_routes(self) -> axum::Router<Self::State>;
}

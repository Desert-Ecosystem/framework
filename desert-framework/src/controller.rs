use std::any::{Any, TypeId};
use std::sync::Arc;

use crate::route::RouteEntry;

pub trait ControllerRoutes: Sized + Send + Sync + 'static {
    const CONTROLLER_PATH: &'static str;

    fn get_router(self) -> axum::Router {
        let state: Arc<dyn Any + Send + Sync> = Arc::new(self);
        let mut router = axum::Router::new();

        for entry in inventory::iter::<RouteEntry> {
            if entry.controller_type_id == TypeId::of::<Self>() {
                let full_path = format!("{}{}", Self::CONTROLLER_PATH, entry.path);
                let mr = (entry.make_route)(state.clone());
                router = router.route(&full_path, mr);
            }
        }

        router
    }
}

use std::any::{Any, TypeId};
use std::sync::Arc;

use axum::routing::MethodRouter;

pub struct RouteEntry {
    pub controller_type_id: TypeId,
    pub path: &'static str,
    pub method: u8,
    pub make_route: fn(Arc<dyn Any + Send + Sync>) -> MethodRouter,
}

inventory::collect!(RouteEntry);

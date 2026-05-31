extern crate self as desert_framework;

pub mod controller;
pub mod dependency;
pub mod macros;
pub mod manager;
pub mod route;
pub mod service;
pub mod test;

pub use controller::ControllerRoutes;
pub use desert_framework_macros::*;
pub use inventory;
pub use route::RouteEntry;

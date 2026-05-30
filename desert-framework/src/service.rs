use std::{sync::Arc, vec};

use crate::{dependency::Deps, manager::DependencyManager};

pub trait Service {
    fn new(manager: Arc<DependencyManager>) -> impl std::future::Future<Output = Self> + Send
    where
        Self: Sized;

    fn deps() -> Deps {
        return vec![];
    }

    fn name() -> String;
}

use std::any::TypeId;

use crate::service::Service;

#[derive(Debug, Clone)]
pub struct Dependency {
    pub type_id: TypeId,
    pub name: String,
}

pub fn dep<T: ?Sized + 'static + Service>() -> Dependency {
    Dependency {
        type_id: TypeId::of::<T>(),
        name: T::name(),
    }
}

pub type Deps = Vec<Dependency>;

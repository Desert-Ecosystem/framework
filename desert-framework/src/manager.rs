use std::{
    any::{Any, TypeId},
    collections::HashMap,
    sync::Arc,
};

use tokio::sync::RwLock;

use crate::service::Service;

#[derive(Debug)]
pub struct DependencyManager {
    dependencies: Arc<RwLock<HashMap<TypeId, Arc<dyn Any + Send + Sync>>>>,
}

impl DependencyManager {
    pub fn new() -> Self {
        Self {
            dependencies: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn register<T: Send + Sync + 'static + Service>(&self) -> Arc<T> {
        let deps = T::deps();

        let mut err_count = 0;

        log::info!("Checking deps {}", T::name());
        for dep in deps {
            if self.check_by_type_id(dep.type_id).await {
                log::info!("> {} ok", dep.name)
            } else {
                err_count += 1;
                log::error!("> {} error", dep.name)
            }
        }

        if err_count > 0 {
            log::error!("failed to create service");
            panic!("failed to create service {}", T::name())
        } else {
            log::info!("deps-check successfull");
        }

        let child_manager = Arc::new(Self {
            dependencies: self.dependencies.clone(),
        });

        let service_instance = Arc::new(T::new(child_manager).await);

        let mut services = self.dependencies.write().await;
        let type_id = TypeId::of::<T>();
        services.insert(
            type_id,
            service_instance.clone() as Arc<dyn Any + Send + Sync>,
        );

        return service_instance;
    }

    async fn check_by_type_id(&self, type_id: TypeId) -> bool {
        let deps = self.dependencies.read().await;
        let got = deps.get(&type_id);
        return got.is_some();
    }

    pub async fn get<T: Send + Sync + 'static + Service>(&self, from: &str) -> Option<Arc<T>> {
        let deps = self.dependencies.read().await;
        let type_id = TypeId::of::<T>();

        let result = deps.get(&type_id).and_then(|arc_any| {
            let cloned = arc_any.clone();
            cloned.downcast::<T>().ok()
        });

        if result.is_none() {
            log::warn!("{} unable to get {} now", from, T::name())
        }

        result
    }
}

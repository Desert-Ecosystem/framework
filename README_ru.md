# Desert Framework

[English version](README.md)

Микрофреймворк для построения backend приложений на Rust с Axum. Предоставляет систему внедрения зависимостей и макросы для декларативного определения маршрутов.

## Установка

```toml
[dependencies]
desert-framework = "0.1.0"
```

## Модули

### Service — система внедрения зависимостей

```rust
use std::sync::Arc;
use desert_framework::{Service, DependencyManager, dep, inject_services};

struct DatabaseService {
    pool: PgPool,
}

impl Service for DatabaseService {
    fn name() -> String { "DatabaseService".into() }

    async fn new(_manager: Arc<DependencyManager>) -> Self {
        Self { pool: PgPool::connect("...").await.unwrap() }
    }
}

struct UserService {
    db: Arc<DatabaseService>,
}

impl Service for UserService {
    fn name() -> String { "UserService".into() }

    fn deps() -> Vec<Dependency> {
        vec![dep::<DatabaseService>()]
    }

    async fn new(manager: Arc<DependencyManager>) -> Self {
        Self { db: manager.get::<DatabaseService>("UserService").await.unwrap() }
    }
}

// Инициализация
let manager = DependencyManager::new();
let db = manager.register::<DatabaseService>().await;
let user_svc = manager.register::<UserService>().await;

// Инъекция через макрос
inject_services!(manager, "MyFunc", {
    db: DatabaseService,
    users: UserService,
});
```

### Controller — макросы для Axum маршрутов

```rust
use desert_framework::{controller, get, post, impl_routes};

#[controller(path = "/api/users")]
struct UserController {
    user_service: Arc<UserService>,
}

impl UserController {
    #[get("/")]
    async fn list(&self) -> Json<Vec<User>> {
        Json(self.user_service.list().await)
    }

    #[get("/{id}")]
    async fn get(&self, Path(id): Path<u64>) -> Json<User> {
        Json(self.user_service.get(id).await)
    }

    #[post("/")]
    async fn create(&self, Json(body): Json<CreateUser>) -> Json<User> {
        Json(self.user_service.create(body).await)
    }
}

impl_routes!(UserController, [list, get, create]);

// Использование в приложении
let controller = UserController { user_service };
let router = controller.get_router();

let app = Router::new()
    .merge(user_controller.get_router())
    .merge(post_controller.get_router());
```

## Макросы

| Макрос | Назначение |
|--------|-----------|
| `#[controller(path = "/prefix")]` | Определяет контроллер с базовым путём |
| `#[get("/path")]` | GET маршрут |
| `#[post("/path")]` | POST маршрут |
| `#[put("/path")]` | PUT маршрут |
| `#[delete("/path")]` | DELETE маршрут |
| `#[patch("/path")]` | PATCH маршрут |
| `impl_routes!(Type, [methods])` | Генерирует `get_router()` для контроллера |
| `inject_services!` | Быстрая инъекция сервисов |

## Параметры маршрутов

```rust
#[get("/items/{id}")]
async fn get_item(&self, Path(id): Path<String>) -> String {
    format!("item: {}", id)
}
```

## Лицензия

MIT

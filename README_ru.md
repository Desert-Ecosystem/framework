# Desert Framework

[English version](README.md)

[![crates.io](https://img.shields.io/crates/v/desert_framework.svg)](https://crates.io/crates/desert_framework)
[![docs.rs](https://docs.rs/desert_framework/badge.svg)](https://docs.rs/desert_framework)

Микрофреймворк для построения backend приложений на Rust с Axum. Предоставляет систему внедрения зависимостей и макросы для декларативного определения маршрутов.

## Установка

```toml
[dependencies]
desert-framework = "*"
```

> Актуальную версию смотрите на [crates.io](https://crates.io/crates/desert_framework).

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
use desert_framework::controller;

#[controller(path = "/api/users")]
struct UserController {
    user_service: Arc<UserService>,
}

#[controller]
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

// Использование в приложении
let controller = UserController { user_service };
let router = controller.get_router();

let app = Router::new()
    .merge(user_controller.get_router())
    .merge(post_controller.get_router());
```

#### Несколько impl блоков

Маршруты обнаруживаются автоматически через `inventory`. Можно разбить методы на несколько `impl` блоков и даже по разным файлам:

```rust
// файл: user_controller.rs
#[controller(path = "/api/users")]
struct UserController { ... }

#[controller]
impl UserController {
    #[get("/")]
    async fn list(&self) -> Json<Vec<User>> { ... }
}

// файл: user_create.rs
#[controller]
impl UserController {
    #[post("/")]
    async fn create(&self, Json(body): Json<CreateUser>) -> Json<User> { ... }
}

// Оба маршрута автоматически попадут в get_router()
```

## Макросы

| Макрос | Назначение |
|--------|-----------|
| `#[controller(path = "/prefix")]` | Определяет контроллер с базовым путём (на struct) |
| `#[controller]` | Обнаруживает route-методы в impl блоке (на impl) |
| `#[get("/path")]` | GET маршрут |
| `#[post("/path")]` | POST маршрут |
| `#[put("/path")]` | PUT маршрут |
| `#[delete("/path")]` | DELETE маршрут |
| `#[patch("/path")]` | PATCH маршрут |
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

# Desert Framework

[Русская версия](README_ru.md)

[![crates.io](https://img.shields.io/crates/v/desert_framework.svg)](https://crates.io/crates/desert_framework)
[![docs.rs](https://docs.rs/desert_framework/badge.svg)](https://docs.rs/desert_framework)

Micro-framework for building backend applications in Rust with Axum. Provides dependency injection system and macros for declarative route definitions.

## Installation

```toml
[dependencies]
desert-framework = "*"
```

> Check [crates.io](https://crates.io/crates/desert_framework) for the latest version.

## Modules

### Service — dependency injection system

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

// Initialization
let manager = DependencyManager::new();
let db = manager.register::<DatabaseService>().await;
let user_svc = manager.register::<UserService>().await;

// Injection via macro
inject_services!(manager, "MyFunc", {
    db: DatabaseService,
    users: UserService,
});
```

### Controller — macros for Axum routes

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

// Usage in application
let controller = UserController { user_service };
let router = controller.get_router();

let app = Router::new()
    .merge(user_controller.get_router())
    .merge(post_controller.get_router());
```

#### Multiple impl blocks

Routes are discovered automatically via `inventory`. You can split methods across multiple `impl` blocks and even multiple files:

```rust
// file: user_controller.rs
#[controller(path = "/api/users")]
struct UserController { ... }

#[controller]
impl UserController {
    #[get("/")]
    async fn list(&self) -> Json<Vec<User>> { ... }
}

// file: user_create.rs
#[controller]
impl UserController {
    #[post("/")]
    async fn create(&self, Json(body): Json<CreateUser>) -> Json<User> { ... }
}

// Both routes are automatically included in get_router()
```

## Macros

| Macro | Description |
|-------|-------------|
| `#[controller(path = "/prefix")]` | Defines controller with base path (on struct) |
| `#[controller]` | Discovers route methods in impl block (on impl) |
| `#[get("/path")]` | GET route |
| `#[post("/path")]` | POST route |
| `#[put("/path")]` | PUT route |
| `#[delete("/path")]` | DELETE route |
| `#[patch("/path")]` | PATCH route |
| `inject_services!` | Quick service injection |

## Route Parameters

```rust
#[get("/items/{id}")]
async fn get_item(&self, Path(id): Path<String>) -> String {
    format!("item: {}", id)
}
```

## License

MIT

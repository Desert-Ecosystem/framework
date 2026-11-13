use utoipa::openapi::path::Paths;
use utoipa::openapi::OpenApi;

pub struct OpenApiEntry {
    pub modify: fn(&mut OpenApi),
}

inventory::collect!(OpenApiEntry);

/// Generates an OpenApi specification incorporating all registered endpoints.
pub fn generate_openapi(mut base: OpenApi) -> OpenApi {
    for entry in inventory::iter::<OpenApiEntry> {
        (entry.modify)(&mut base);
    }
    base
}

/// Helper to register a Path into OpenApi.
pub fn register_path<'t, P: utoipa::Path + utoipa::__dev::Tags<'t> + utoipa::__dev::SchemaReferences>(
    openapi: &mut OpenApi,
) {
    if openapi.paths.paths.is_empty() && openapi.paths.extensions.is_none() {
        openapi.paths = Paths::new();
    }

    let path_str = P::path();
    let methods = P::methods();
    let mut operation = P::operation();

    // utoipa generates Tags<'t> for each path item, but P::operation() does not attach tags
    // unless PathConfig / operation tags are explicitly populated.
    let tags = P::tags();
    if !tags.is_empty() {
        let op_tags = operation.tags.get_or_insert_with(Vec::new);
        for tag in tags {
            if !op_tags.iter().any(|t| t == tag) {
                op_tags.push(tag.to_string());
            }
        }
    }

    openapi
        .paths
        .add_path_operation(path_str, methods, operation);

    let components = openapi
        .components
        .get_or_insert_with(utoipa::openapi::Components::new);
    let mut schemas = Vec::new();
    <P as utoipa::__dev::SchemaReferences>::schemas(&mut schemas);
    components.schemas.extend(schemas);
}

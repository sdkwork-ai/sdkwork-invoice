pub mod command_headers;
pub mod http_route_manifest;
pub mod invoice_router;
pub mod routes;
pub mod subject;
pub mod web_bootstrap;

pub use http_route_manifest::app_route_manifest;
pub use invoice_router::{
    app_invoice_router_with_postgres_pool, app_invoice_router_with_sqlite_pool,
    build_app_invoice_router, CommerceInvoiceFuture, CommerceInvoiceStore,
};
pub use routes::{build_invoice_app_router, build_invoice_app_router_with_framework};
pub use web_bootstrap::wrap_router_with_web_framework_from_env;

use axum::Router;
use sdkwork_invoice_service_host::InvoiceServiceHost;
use sdkwork_web_core::HttpRouteManifest;
use std::sync::Arc;

pub fn gateway_route_manifest() -> HttpRouteManifest {
    app_route_manifest()
}

pub async fn gateway_mount(host: Arc<InvoiceServiceHost>) -> Router {
    build_invoice_app_router_with_framework(host).await
}

pub fn gateway_mount_business(host: Arc<InvoiceServiceHost>) -> Router {
    build_invoice_app_router(host)
}

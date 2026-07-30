use axum::Router;
use sdkwork_invoice_service_host::InvoiceServiceHost;
use std::sync::Arc;

use crate::app_invoice_router_with_postgres_pool;
use crate::web_bootstrap::wrap_router_with_web_framework_from_env;

pub fn build_invoice_app_router(host: Arc<InvoiceServiceHost>) -> Router {
    let pool = host
        .database_pool()
        .as_postgres()
        .expect("invoice app-api requires an authoritative PostgreSQL pool");
    app_invoice_router_with_postgres_pool(pool.clone())
}

pub async fn build_invoice_app_router_with_framework(host: Arc<InvoiceServiceHost>) -> Router {
    wrap_router_with_web_framework_from_env(build_invoice_app_router(host)).await
}

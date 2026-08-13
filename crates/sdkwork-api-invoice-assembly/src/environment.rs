use std::sync::Arc;

use sdkwork_database_sqlx::DatabasePool;

use sdkwork_invoice_service_host::InvoiceServiceHost;
use sdkwork_web_bootstrap::DatabasePoolReadinessCheck;

use crate::bootstrap::{
    assemble_api_router, assemble_app_api_contribution, ApiAssembly, ApiAssemblyContext,
};

async fn context_from_env() -> Result<ApiAssemblyContext, String> {
    let host = Arc::new(InvoiceServiceHost::from_env().await?);
    let readiness_check = Arc::new(DatabasePoolReadinessCheck::new(
        host.database_pool().clone(),
    ));
    Ok(ApiAssemblyContext {
        host,
        domain_context_injectors: Vec::new(),
        readiness_check,
    })
}


async fn context_from_pool(pool: DatabasePool) -> Result<ApiAssemblyContext, String> {
    let host = Arc::new(InvoiceServiceHost::from_pool(pool).await?);
    let readiness_check = Arc::new(DatabasePoolReadinessCheck::new(
        host.database_pool().clone(),
    ));
    Ok(ApiAssemblyContext {
        host,
        domain_context_injectors: Vec::new(),
        readiness_check,
    })
}

/// Assemble the full invoice router against a caller-provided database pool so
/// the platform cloud gateway can share its process-wide PostgreSQL pool.
pub async fn assemble_api_router_with_pool(pool: DatabasePool) -> Result<ApiAssembly, String> {
    assemble_api_router(context_from_pool(pool).await?).await
}

pub async fn assemble_api_router_from_env() -> Result<ApiAssembly, String> {
    assemble_api_router(context_from_env().await?).await
}

pub async fn assemble_app_api_contribution_from_env() -> Result<ApiAssembly, String> {
    assemble_app_api_contribution(context_from_env().await?).await
}

/// Same-origin dependency composition: build the invoice App API contribution
/// on a shared pool owned by the consuming host.
pub async fn assemble_app_api_contribution_with_pool(
    pool: DatabasePool,
) -> Result<ApiAssembly, String> {
    assemble_app_api_contribution(context_from_pool(pool).await?).await
}

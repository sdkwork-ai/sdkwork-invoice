use std::sync::Arc;

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

pub async fn assemble_api_router_from_env() -> Result<ApiAssembly, String> {
    assemble_api_router(context_from_env().await?).await
}

pub async fn assemble_app_api_contribution_from_env() -> Result<ApiAssembly, String> {
    assemble_app_api_contribution(context_from_env().await?).await
}

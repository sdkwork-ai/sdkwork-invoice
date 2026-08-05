use sdkwork_database_config::DatabaseConfig;
use sdkwork_database_lifecycle::{lifecycle_options_from_env, LifecycleOrchestrator};
use sdkwork_database_spi::{DatabaseAssetProvider, DatabaseManifest, DefaultDatabaseModule};
use sdkwork_database_sqlx::{create_pool_from_config, DatabasePool};
use std::path::PathBuf;
use std::sync::Arc;

pub struct InvoiceDatabaseHost {
    pool: DatabasePool,
    module: Arc<DefaultDatabaseModule>,
}

impl InvoiceDatabaseHost {
    pub fn pool(&self) -> &DatabasePool {
        &self.pool
    }

    pub fn module(&self) -> Arc<DefaultDatabaseModule> {
        self.module.clone()
    }
}

pub async fn bootstrap_invoice_database_from_env() -> Result<InvoiceDatabaseHost, String> {
    let _ = dotenvy::dotenv();
    let config = DatabaseConfig::from_env("INVOICE")
        .map_err(|error| format!("read invoice database config failed: {error}"))?;
    let pool = create_pool_from_config(config)
        .await
        .map_err(|error| format!("create invoice database pool failed: {error}"))?;
    bootstrap_invoice_database_with_pool(pool).await
}

/// Bootstrap invoice assets against a caller-provided database pool so the
/// platform cloud gateway can share its process-wide PostgreSQL pool.
pub async fn bootstrap_invoice_database_with_pool(
    pool: DatabasePool,
) -> Result<InvoiceDatabaseHost, String> {
    let app_root = std::env::var("SDKWORK_INVOICE_APP_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.."));
    let module = Arc::new(
        DefaultDatabaseModule::from_app_root(&app_root)
            .map_err(|error| format!("load invoice database module failed: {error}"))?,
    );
    let manifest = DatabaseManifest::from_file(module.manifest_path())
        .map_err(|error| format!("read invoice database manifest failed: {error}"))?;
    let options = lifecycle_options_from_env("INVOICE", &manifest);
    let orchestrator =
        LifecycleOrchestrator::new(pool.clone(), module.clone()).with_applied_by("sdkwork-invoice");
    orchestrator.init().await.map_err(|e| format!("{e}"))?;
    if options.auto_migrate {
        orchestrator.migrate().await.map_err(|e| format!("{e}"))?;
    }
    Ok(InvoiceDatabaseHost { pool, module })
}

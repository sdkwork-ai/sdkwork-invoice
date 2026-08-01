use sdkwork_api_invoice_assembly::assemble_api_router_from_env;
use sdkwork_web_bootstrap::{service_router, ServiceRouterConfig};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    let business = match assemble_api_router_from_env().await { Ok(api) => api.router, Err(error) => { tracing::error!(%error, "invoice assembly failed"); std::process::exit(1); } }.layer(
        sdkwork_web_bootstrap::application_cors_layer_from_env(
            &["SDKWORK_INVOICE_ENVIRONMENT"],
            &[
                "SDKWORK_INVOICE_CORS_ALLOWED_ORIGINS",
                "SDKWORK_CORS_ALLOWED_ORIGINS",
            ],
        ),
    );
    let app = service_router(business, ServiceRouterConfig::default().with_always_ready());
    let addr = std::env::var("INVOICE_API_BIND").unwrap_or_else(|_| "0.0.0.0:18098".to_owned());
    let listener = tokio::net::TcpListener::bind(&addr).await.expect("bind");
    axum::serve(listener, app).await.expect("serve");
}

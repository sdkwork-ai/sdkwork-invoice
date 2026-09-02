//! API assembly for sdkwork-invoice.
//! Application bootstrap lives in `bootstrap.rs`; route inventory is in `assembly-manifest.json`.
// SDKWORK-ASSEMBLY-LIB-CUSTOM

mod bootstrap;
mod environment;
mod generated;

pub use bootstrap::{
    assemble_api_router, assemble_app_api_contribution, web_module_with_context, ApiAssembly,
    ApiAssemblyContext,
};
pub use environment::{
    assemble_api_router_from_env, assemble_api_router_runtime, assemble_api_router_with_pool,
    assemble_app_api_contribution_from_env, assemble_app_api_contribution_with_pool, web_module,
    web_module_with_pool, ApiAssemblyRuntime,
};
/// App-api surface route manifest owned by the dependency assembly.
pub fn app_api_route_manifest() -> sdkwork_web_core::HttpRouteManifest {
    sdkwork_routes_invoice_app_api::app_route_manifest()
}

pub fn assembly_route_count() -> usize {
    generated::ROUTE_CRATE_COUNT
}

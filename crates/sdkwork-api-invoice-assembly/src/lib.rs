//! API assembly for sdkwork-invoice.
//! Application bootstrap lives in `bootstrap.rs`; route inventory is in `assembly-manifest.json`.
// SDKWORK-ASSEMBLY-LIB-CUSTOM

mod bootstrap;
mod environment;
mod generated;

pub use bootstrap::{
    assemble_api_router, assemble_app_api_contribution, ApiAssembly, ApiAssemblyContext,
};
pub use environment::{assemble_api_router_from_env, assemble_app_api_contribution_from_env};

pub fn assembly_route_count() -> usize {
    generated::ROUTE_CRATE_COUNT
}

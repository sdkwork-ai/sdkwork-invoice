//! API assembly for sdkwork-invoice.
//! Application bootstrap lives in `bootstrap.rs`; route inventory is in `assembly-manifest.json`.

mod bootstrap;
mod generated;

pub use bootstrap::{assemble_api_router, assemble_api_router_from_env, ApiAssembly};

pub fn assembly_route_count() -> usize {
    generated::ROUTE_CRATE_COUNT
}

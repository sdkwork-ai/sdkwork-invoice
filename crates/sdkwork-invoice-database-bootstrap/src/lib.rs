//! Invoice database bootstrap authority for the `sdkwork-invoice` product.
//!
//! Host installers (including sdkwork-cloudrouter standalone mode) must consume this
//! crate instead of embedding invoice DDL paths in local compatibility shims.

/// Canonical Postgres baseline for invoice tables owned by `sdkwork-invoice`.
pub fn invoice_foundation_migration_sql() -> &'static str {
    include_str!("../../../database/ddl/baseline/postgres/0001_invoice_baseline.sql")
}

/// SQLite mirror of the invoice module baseline for host installers and drift checks.
pub fn invoice_foundation_migration_sqlite() -> &'static str {
    include_str!("../../../tests/fixtures/database/sqlite/ddl/baseline/0001_invoice_baseline.sql")
}

/// Tables that must exist after the invoice module baseline is applied.
pub fn invoice_module_table_names() -> Vec<&'static str> {
    vec![
        "commerce_invoice_title",
        "commerce_invoice",
        "commerce_invoice_item",
    ]
}

use std::collections::HashMap;

use sdkwork_contract_service::CommerceServiceError;
use sdkwork_invoice_service::{
    CancelOwnerInvoiceCommand, CreateOwnerInvoiceCommand, InvoiceDetailQuery, InvoiceItemListPage,
    InvoiceItemListQuery, InvoiceItemRecord, InvoiceListPage, InvoiceListQuery, InvoiceRecord,
    InvoiceStatistics, SubmitOwnerInvoiceCommand, UpdateOwnerInvoiceCommand,
};
use sqlx::{PgPool, Postgres, QueryBuilder, Row};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone)]
pub struct PostgresCommerceInvoiceStore {
    pool: PgPool,
}

#[derive(Debug, Clone)]
struct InvoiceRow {
    id: String,
    tenant_id: String,
    organization_id: Option<String>,
    owner_user_id: String,
    order_id: String,
    payment_id: String,
    title_id: String,
    status: String,
    invoice_no: Option<String>,
    invoice_code: Option<String>,
    document_url: Option<String>,
    created_at: String,
    issued_at: Option<String>,
    updated_at: String,
}

impl PostgresCommerceInvoiceStore {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn list_invoices(
        &self,
        query: InvoiceListQuery,
    ) -> Result<InvoiceListPage, CommerceServiceError> {
        let total = count_invoices(&self.pool, &query).await?;
        let rows = sqlx::query(
            r#"
            SELECT id, tenant_id, organization_id, owner_user_id, order_id, payment_id,
                   title_id, status, invoice_no, invoice_code, document_url,
                   created_at, issued_at, updated_at
            FROM commerce_invoice
            WHERE tenant_id = CAST($1 AS TEXT)
              AND ((organization_id = CAST($2 AS TEXT)) OR (organization_id IS NULL AND $2 IS NULL) OR (organization_id = '0' AND $2 IS NULL))
              AND owner_user_id = CAST($3 AS TEXT)
              AND ($4 IS NULL OR status = $4)
            ORDER BY COALESCE(issued_at, created_at) DESC NULLS LAST, id DESC
            LIMIT $5 OFFSET $6
            "#,
        )
        .bind(&query.tenant_id)
        .bind(query.organization_id.as_deref())
        .bind(&query.owner_user_id)
        .bind(query.status.as_deref())
        .bind(query.limit())
        .bind(query.offset())
        .fetch_all(&self.pool)
        .await
        .map_err(|error| store_error("failed to list invoices", error))?;

        let invoice_rows = rows.iter().map(map_invoice_row).collect::<Vec<_>>();
        let items_by_invoice =
            load_items_by_invoice(&self.pool, &query.tenant_id, invoice_rows.as_slice()).await?;
        let invoices = invoice_rows
            .into_iter()
            .map(|row| invoice_from_row(row, &items_by_invoice))
            .collect::<Result<Vec<_>, _>>()?;

        InvoiceListPage::new(invoices, total, query.page_no(), query.limit())
    }

    pub async fn retrieve_invoice(
        &self,
        query: InvoiceDetailQuery,
    ) -> Result<Option<InvoiceRecord>, CommerceServiceError> {
        let row = sqlx::query(
            r#"
            SELECT id, tenant_id, organization_id, owner_user_id, order_id, payment_id,
                   title_id, status, invoice_no, invoice_code, document_url,
                   created_at, issued_at, updated_at
            FROM commerce_invoice
            WHERE tenant_id = CAST($1 AS TEXT)
              AND ((organization_id = CAST($2 AS TEXT)) OR (organization_id IS NULL AND $2 IS NULL) OR (organization_id = '0' AND $2 IS NULL))
              AND owner_user_id = CAST($3 AS TEXT)
              AND id = CAST($4 AS TEXT)
            "#,
        )
        .bind(&query.tenant_id)
        .bind(query.organization_id.as_deref())
        .bind(&query.owner_user_id)
        .bind(&query.invoice_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|error| store_error("failed to retrieve invoice", error))?;

        let Some(row) = row else {
            return Ok(None);
        };
        let invoice_row = map_invoice_row(&row);
        let items_by_invoice = load_items_by_invoice(
            &self.pool,
            &query.tenant_id,
            std::slice::from_ref(&invoice_row),
        )
        .await?;

        invoice_from_row(invoice_row, &items_by_invoice).map(Some)
    }

    pub async fn invoice_statistics(
        &self,
        query: InvoiceListQuery,
    ) -> Result<InvoiceStatistics, CommerceServiceError> {
        let row = sqlx::query(
            r#"
            SELECT COUNT(*) AS total,
                   COALESCE(SUM(CASE WHEN LOWER(status) IN ('issued', 'completed') THEN 1 ELSE 0 END), 0)::BIGINT AS issued,
                   COALESCE(SUM(CASE WHEN LOWER(status) IN ('cancelled', 'canceled') THEN 1 ELSE 0 END), 0)::BIGINT AS cancelled,
                   COALESCE(SUM(CASE WHEN LOWER(status) NOT IN ('issued', 'completed', 'cancelled', 'canceled') THEN 1 ELSE 0 END), 0)::BIGINT AS pending
            FROM commerce_invoice
            WHERE tenant_id = CAST($1 AS TEXT)
              AND ((organization_id = CAST($2 AS TEXT)) OR (organization_id IS NULL AND $2 IS NULL) OR (organization_id = '0' AND $2 IS NULL))
              AND owner_user_id = CAST($3 AS TEXT)
            "#,
        )
        .bind(&query.tenant_id)
        .bind(query.organization_id.as_deref())
        .bind(&query.owner_user_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|error| store_error("failed to aggregate invoice statistics", error))?;
        Ok(InvoiceStatistics {
            total: row.try_get("total").unwrap_or(0),
            pending: row.try_get("pending").unwrap_or(0),
            issued: row.try_get("issued").unwrap_or(0),
            cancelled: row.try_get("cancelled").unwrap_or(0),
        })
    }

    pub async fn list_invoice_items(
        &self,
        query: InvoiceItemListQuery,
    ) -> Result<InvoiceItemListPage, CommerceServiceError> {
        let total: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*)
            FROM commerce_invoice_item item
            JOIN commerce_invoice invoice
              ON invoice.id = item.invoice_id AND invoice.tenant_id = item.tenant_id
            WHERE invoice.tenant_id = CAST($1 AS TEXT)
              AND ((invoice.organization_id = CAST($2 AS TEXT)) OR (invoice.organization_id IS NULL AND $2 IS NULL) OR (invoice.organization_id = '0' AND $2 IS NULL))
              AND invoice.owner_user_id = CAST($3 AS TEXT)
              AND invoice.id = CAST($4 AS TEXT)
            "#,
        )
        .bind(&query.tenant_id)
        .bind(query.organization_id.as_deref())
        .bind(&query.owner_user_id)
        .bind(&query.invoice_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|error| store_error("failed to count invoice items", error))?;
        let rows = sqlx::query(
            r#"
            SELECT item.id, item.tenant_id, item.invoice_id, item.order_item_id, item.title,
                   item.amount, item.tax_amount, item.created_at
            FROM commerce_invoice_item item
            JOIN commerce_invoice invoice
              ON invoice.id = item.invoice_id AND invoice.tenant_id = item.tenant_id
            WHERE invoice.tenant_id = CAST($1 AS TEXT)
              AND ((invoice.organization_id = CAST($2 AS TEXT)) OR (invoice.organization_id IS NULL AND $2 IS NULL) OR (invoice.organization_id = '0' AND $2 IS NULL))
              AND invoice.owner_user_id = CAST($3 AS TEXT)
              AND invoice.id = CAST($4 AS TEXT)
            ORDER BY item.created_at ASC, item.id ASC
            LIMIT $5 OFFSET $6
            "#,
        )
        .bind(&query.tenant_id)
        .bind(query.organization_id.as_deref())
        .bind(&query.owner_user_id)
        .bind(&query.invoice_id)
        .bind(query.page_size)
        .bind(query.offset())
        .fetch_all(&self.pool)
        .await
        .map_err(|error| store_error("failed to list invoice items", error))?;
        let items = rows
            .iter()
            .map(invoice_item_from_row)
            .collect::<Result<Vec<_>, _>>()?;
        InvoiceItemListPage::new(items, total, query.page, query.page_size)
    }

    pub async fn create_owner_invoice(
        &self,
        command: CreateOwnerInvoiceCommand,
    ) -> Result<InvoiceRecord, CommerceServiceError> {
        let now = invoice_command_timestamp();
        let invoice_id = format!("invoice-{now}");
        let title_id = format!("title-{now}");
        let item_id = format!("invoice-item-{now}");
        let order_id = format!("manual-{invoice_id}");
        let payment_id = format!("manual-payment-{invoice_id}");

        let mut tx =
            self.pool.begin().await.map_err(|error| {
                store_error("failed to begin create invoice transaction", error)
            })?;

        sqlx::query(
            r#"
            INSERT INTO commerce_invoice_title
                (id, tenant_id, owner_user_id, title_type, name, tax_no, created_at, updated_at)
            VALUES ($1, CAST($2 AS TEXT), CAST($3 AS TEXT), $4, $5, $6, $7, $8)
            "#,
        )
        .bind(&title_id)
        .bind(&command.tenant_id)
        .bind(&command.owner_user_id)
        .bind(&command.title_type)
        .bind(&command.title)
        .bind(command.tax_no.as_deref())
        .bind(&now)
        .bind(&now)
        .execute(&mut *tx)
        .await
        .map_err(|error| store_error("failed to insert invoice title", error))?;

        sqlx::query(
            r#"
            INSERT INTO commerce_invoice
                (id, tenant_id, organization_id, owner_user_id, order_id, payment_id, title_id,
                 status, invoice_no, invoice_code, document_url, created_at, issued_at, updated_at)
            VALUES
                ($1, CAST($2 AS TEXT), CAST($3 AS TEXT), CAST($4 AS TEXT), $5, $6, $7, 'draft', NULL, NULL, NULL, $8, NULL, $9)
            "#,
        )
        .bind(&invoice_id)
        .bind(&command.tenant_id)
        .bind(command.organization_id.as_deref())
        .bind(&command.owner_user_id)
        .bind(&order_id)
        .bind(&payment_id)
        .bind(&title_id)
        .bind(&now)
        .bind(&now)
        .execute(&mut *tx)
        .await
        .map_err(|error| store_error("failed to insert invoice", error))?;

        sqlx::query(
            r#"
            INSERT INTO commerce_invoice_item
                (id, tenant_id, invoice_id, order_item_id, title, amount, tax_amount, created_at)
            VALUES ($1, CAST($2 AS TEXT), $3, NULL, $4, $5, '0.00', $6)
            "#,
        )
        .bind(&item_id)
        .bind(&command.tenant_id)
        .bind(&invoice_id)
        .bind(&command.title)
        .bind(&command.total_amount)
        .bind(&now)
        .execute(&mut *tx)
        .await
        .map_err(|error| store_error("failed to insert invoice item", error))?;

        tx.commit()
            .await
            .map_err(|error| store_error("failed to commit create invoice transaction", error))?;

        let query = InvoiceDetailQuery::new(
            &command.tenant_id,
            command.organization_id.as_deref(),
            &command.owner_user_id,
            &invoice_id,
        )?;
        self.retrieve_invoice(query)
            .await?
            .ok_or_else(|| CommerceServiceError::storage("created invoice was not readable"))
    }

    pub async fn submit_owner_invoice(
        &self,
        command: SubmitOwnerInvoiceCommand,
    ) -> Result<InvoiceRecord, CommerceServiceError> {
        let now = invoice_command_timestamp();
        let updated = sqlx::query(
            r#"
            UPDATE commerce_invoice
            SET status = 'submitted', updated_at = $1
            WHERE tenant_id = CAST($2 AS TEXT)
              AND ((organization_id = CAST($3 AS TEXT)) OR (organization_id IS NULL AND $4 IS NULL) OR (organization_id = '0' AND $4 IS NULL))
              AND owner_user_id = CAST($5 AS TEXT)
              AND id = CAST($6 AS TEXT)
              AND LOWER(COALESCE(status, '')) IN ('draft', 'failed')
            "#,
        )
        .bind(&now)
        .bind(&command.tenant_id)
        .bind(command.organization_id.as_deref())
        .bind(command.organization_id.as_deref())
        .bind(&command.owner_user_id)
        .bind(&command.invoice_id)
        .execute(&self.pool)
        .await
        .map_err(|error| store_error("failed to submit invoice", error))?;

        if updated.rows_affected() == 0 {
            return Err(CommerceServiceError::conflict(
                "invoice is not submittable or was not found",
            ));
        }

        let query = InvoiceDetailQuery::new(
            &command.tenant_id,
            command.organization_id.as_deref(),
            &command.owner_user_id,
            &command.invoice_id,
        )?;
        self.retrieve_invoice(query)
            .await?
            .ok_or_else(|| CommerceServiceError::not_found("invoice was not found"))
    }

    pub async fn cancel_owner_invoice(
        &self,
        command: CancelOwnerInvoiceCommand,
    ) -> Result<(), CommerceServiceError> {
        let now = invoice_command_timestamp();
        let updated = sqlx::query(
            r#"
            UPDATE commerce_invoice
            SET status = 'cancelled', updated_at = $1
            WHERE tenant_id = CAST($2 AS TEXT)
              AND ((organization_id = CAST($3 AS TEXT)) OR (organization_id IS NULL AND $4 IS NULL) OR (organization_id = '0' AND $4 IS NULL))
              AND owner_user_id = CAST($5 AS TEXT)
              AND id = CAST($6 AS TEXT)
              AND LOWER(COALESCE(status, '')) IN ('issued', 'completed')
            "#,
        )
        .bind(&now)
        .bind(&command.tenant_id)
        .bind(command.organization_id.as_deref())
        .bind(command.organization_id.as_deref())
        .bind(&command.owner_user_id)
        .bind(&command.invoice_id)
        .execute(&self.pool)
        .await
        .map_err(|error| store_error("failed to cancel invoice", error))?;

        if updated.rows_affected() == 0 {
            return Err(CommerceServiceError::conflict(
                "invoice is not cancellable or was not found",
            ));
        }

        let _ = command.cancel_reason;
        Ok(())
    }

    pub async fn update_owner_invoice(
        &self,
        command: UpdateOwnerInvoiceCommand,
    ) -> Result<InvoiceRecord, CommerceServiceError> {
        let query = InvoiceDetailQuery::new(
            &command.tenant_id,
            command.organization_id.as_deref(),
            &command.owner_user_id,
            &command.invoice_id,
        )?;
        let Some(existing) = self.retrieve_invoice(query.clone()).await? else {
            return Err(CommerceServiceError::not_found("invoice was not found"));
        };
        if !matches!(
            existing.status.to_ascii_lowercase().as_str(),
            "draft" | "failed"
        ) {
            return Err(CommerceServiceError::conflict(
                "invoice is not editable in its current status",
            ));
        }

        let now = invoice_command_timestamp();
        if command.title.is_some() || command.tax_no.is_some() {
            sqlx::query(
                r#"
                UPDATE commerce_invoice_title
                SET name = COALESCE($1, name),
                    tax_no = COALESCE($2, tax_no),
                    updated_at = $3
                WHERE id = $4
                  AND tenant_id = CAST($5 AS TEXT)
                  AND owner_user_id = CAST($6 AS TEXT)
                "#,
            )
            .bind(command.title.as_deref())
            .bind(command.tax_no.as_deref())
            .bind(&now)
            .bind(&existing.title_id)
            .bind(&command.tenant_id)
            .bind(&command.owner_user_id)
            .execute(&self.pool)
            .await
            .map_err(|error| store_error("failed to update invoice title", error))?;
        }

        sqlx::query(
            r#"
            UPDATE commerce_invoice
            SET updated_at = $1
            WHERE id = $2
              AND tenant_id = CAST($3 AS TEXT)
              AND owner_user_id = CAST($4 AS TEXT)
            "#,
        )
        .bind(&now)
        .bind(&command.invoice_id)
        .bind(&command.tenant_id)
        .bind(&command.owner_user_id)
        .execute(&self.pool)
        .await
        .map_err(|error| store_error("failed to update invoice", error))?;

        let _ = (
            command.bank_account,
            command.bank_name,
            command.register_address,
            command.register_phone,
        );
        self.retrieve_invoice(query)
            .await?
            .ok_or_else(|| CommerceServiceError::not_found("invoice was not found"))
    }
}

async fn count_invoices(
    pool: &PgPool,
    query: &InvoiceListQuery,
) -> Result<i64, CommerceServiceError> {
    sqlx::query_scalar(
        r#"
        SELECT COUNT(1)
        FROM commerce_invoice
        WHERE tenant_id = CAST($1 AS TEXT)
          AND ((organization_id = CAST($2 AS TEXT)) OR (organization_id IS NULL AND $2 IS NULL) OR (organization_id = '0' AND $2 IS NULL))
          AND owner_user_id = CAST($3 AS TEXT)
          AND ($4 IS NULL OR status = $4)
        "#,
    )
    .bind(&query.tenant_id)
    .bind(query.organization_id.as_deref())
    .bind(&query.owner_user_id)
    .bind(query.status.as_deref())
    .fetch_one(pool)
    .await
    .map_err(|error| store_error("failed to count invoices", error))
}

async fn load_items_by_invoice(
    pool: &PgPool,
    tenant_id: &str,
    invoices: &[InvoiceRow],
) -> Result<HashMap<String, Vec<InvoiceItemRecord>>, CommerceServiceError> {
    if invoices.is_empty() {
        return Ok(HashMap::new());
    }

    let mut builder = QueryBuilder::<Postgres>::new(
        "SELECT id, tenant_id, invoice_id, order_item_id, title, amount, tax_amount, created_at \
         FROM commerce_invoice_item WHERE tenant_id = ",
    );
    builder.push_bind(tenant_id);
    builder.push(" AND invoice_id IN (");
    {
        let mut separated = builder.separated(", ");
        for invoice in invoices {
            separated.push_bind(&invoice.id);
        }
        separated.push_unseparated(")");
    }
    builder.push(" ORDER BY created_at ASC, id ASC");

    let rows = builder
        .build()
        .fetch_all(pool)
        .await
        .map_err(|error| store_error("failed to list invoice items", error))?;

    let mut items_by_invoice: HashMap<String, Vec<InvoiceItemRecord>> = HashMap::new();
    for row in rows {
        let item = invoice_item_from_row(&row)?;
        let invoice_id = item.invoice_id.clone();
        items_by_invoice.entry(invoice_id).or_default().push(item);
    }
    Ok(items_by_invoice)
}

fn invoice_item_from_row(
    row: &sqlx::postgres::PgRow,
) -> Result<InvoiceItemRecord, CommerceServiceError> {
    InvoiceItemRecord::new(
        &string_cell(row, "id"),
        &string_cell(row, "tenant_id"),
        &string_cell(row, "invoice_id"),
        optional_string_cell(row, "order_item_id").as_deref(),
        &string_cell(row, "title"),
        &string_cell(row, "amount"),
        &string_cell(row, "tax_amount"),
        &string_cell(row, "created_at"),
    )
}

fn invoice_from_row(
    row: InvoiceRow,
    items_by_invoice: &HashMap<String, Vec<InvoiceItemRecord>>,
) -> Result<InvoiceRecord, CommerceServiceError> {
    let items = items_by_invoice
        .get(&row.id)
        .cloned()
        .unwrap_or_else(Vec::new);
    InvoiceRecord::new(
        &row.id,
        &row.tenant_id,
        row.organization_id.as_deref(),
        &row.owner_user_id,
        &row.order_id,
        &row.payment_id,
        &row.title_id,
        &row.status,
        row.invoice_no.as_deref(),
        row.invoice_code.as_deref(),
        row.document_url.as_deref(),
        &row.created_at,
        row.issued_at.as_deref(),
        &row.updated_at,
        items,
    )
}

fn map_invoice_row(row: &sqlx::postgres::PgRow) -> InvoiceRow {
    InvoiceRow {
        id: string_cell(row, "id"),
        tenant_id: string_cell(row, "tenant_id"),
        organization_id: optional_string_cell(row, "organization_id"),
        owner_user_id: string_cell(row, "owner_user_id"),
        order_id: string_cell(row, "order_id"),
        payment_id: string_cell(row, "payment_id"),
        title_id: string_cell(row, "title_id"),
        status: string_cell(row, "status"),
        invoice_no: optional_string_cell(row, "invoice_no"),
        invoice_code: optional_string_cell(row, "invoice_code"),
        document_url: optional_string_cell(row, "document_url"),
        created_at: string_cell(row, "created_at"),
        issued_at: optional_string_cell(row, "issued_at"),
        updated_at: string_cell(row, "updated_at"),
    }
}

fn string_cell(row: &sqlx::postgres::PgRow, name: &str) -> String {
    row.try_get::<String, _>(name).unwrap_or_default()
}

fn optional_string_cell(row: &sqlx::postgres::PgRow, name: &str) -> Option<String> {
    row.try_get::<Option<String>, _>(name)
        .ok()
        .flatten()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn invoice_command_timestamp() -> String {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or(0);
    format!("{seconds}")
}

fn store_error(context: &str, error: sqlx::Error) -> CommerceServiceError {
    CommerceServiceError::storage(format!("{context}: {error}"))
}

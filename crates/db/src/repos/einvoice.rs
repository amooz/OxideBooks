use oxidebooks_core::models::{
    CreateBillLine, CreateVendorBill, EInvoiceTransmission, InboundEInvoice, SendEInvoice,
};
use sqlx::PgPool;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::error::{map_sqlx_err, DbError};
use crate::repos::BillRepo;

fn parse_uuid(s: &str) -> Result<Uuid, DbError> {
    Uuid::parse_str(s).map_err(|_| DbError::Conflict(format!("invalid UUID: {s}")))
}

#[derive(sqlx::FromRow)]
struct TransmissionRow {
    id: Uuid,
    organization_id: Uuid,
    invoice_id: Uuid,
    format: String,
    status: String,
    external_id: Option<String>,
    transmission_xml: Option<String>,
    error_message: Option<String>,
    sent_at: Option<OffsetDateTime>,
    created_at: OffsetDateTime,
    updated_at: OffsetDateTime,
}

impl From<TransmissionRow> for EInvoiceTransmission {
    fn from(r: TransmissionRow) -> Self {
        EInvoiceTransmission {
            id: r.id.to_string(),
            organization_id: r.organization_id.to_string(),
            invoice_id: r.invoice_id.to_string(),
            format: r.format,
            status: r.status,
            external_id: r.external_id,
            transmission_xml: r.transmission_xml,
            error_message: r.error_message,
            sent_at: r.sent_at,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

// ─── UBL 2.1 XML builder ──────────────────────────────────────────────────────

struct InvoiceData {
    invoice_number: String,
    invoice_date: String,
    due_date: String,
    currency: String,
    seller_name: String,
    seller_tax_number: Option<String>,
    buyer_name: String,
    buyer_tax_number: Option<String>,
    lines: Vec<LineData>,
    total_amount: i64,
}

struct LineData {
    id: usize,
    description: String,
    quantity: i64,
    unit_price: i64,
    tax_rate: i64,
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

fn build_ubl_xml(d: &InvoiceData) -> String {
    let mut lines_xml = String::new();
    for ln in &d.lines {
        let qty = ln.quantity as f64 / 100.0;
        let price = ln.unit_price as f64 / 100.0;
        let tax_pct = ln.tax_rate as f64 / 100.0;
        let line_ext = qty * price;
        let tax_amt = line_ext * tax_pct / 100.0;
        lines_xml.push_str(&format!(
            r#"    <cac:InvoiceLine>
      <cbc:ID>{}</cbc:ID>
      <cbc:InvoicedQuantity unitCode="EA">{:.2}</cbc:InvoicedQuantity>
      <cbc:LineExtensionAmount currencyID="{}">{:.2}</cbc:LineExtensionAmount>
      <cac:TaxTotal>
        <cbc:TaxAmount currencyID="{}">{:.2}</cbc:TaxAmount>
      </cac:TaxTotal>
      <cac:Item>
        <cbc:Description>{}</cbc:Description>
      </cac:Item>
      <cac:Price>
        <cbc:PriceAmount currencyID="{}">{:.2}</cbc:PriceAmount>
      </cac:Price>
    </cac:InvoiceLine>
"#,
            ln.id,
            qty,
            xml_escape(&d.currency),
            line_ext,
            xml_escape(&d.currency),
            tax_amt,
            xml_escape(&ln.description),
            xml_escape(&d.currency),
            price,
        ));
    }

    let seller_tax = d
        .seller_tax_number
        .as_deref()
        .map(|t| format!("<cbc:CompanyID>{}</cbc:CompanyID>", xml_escape(t)))
        .unwrap_or_default();
    let buyer_tax = d
        .buyer_tax_number
        .as_deref()
        .map(|t| format!("<cbc:CompanyID>{}</cbc:CompanyID>", xml_escape(t)))
        .unwrap_or_default();
    let total = d.total_amount as f64 / 100.0;

    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<Invoice xmlns="urn:oasis:names:specification:ubl:schema:xsd:Invoice-2"
         xmlns:cac="urn:oasis:names:specification:ubl:schema:xsd:CommonAggregateComponents-2"
         xmlns:cbc="urn:oasis:names:specification:ubl:schema:xsd:CommonBasicComponents-2">
  <cbc:UBLVersionID>2.1</cbc:UBLVersionID>
  <cbc:CustomizationID>urn:cen.eu:en16931:2017#compliant#urn:fdc:peppol.eu:2017:poacc:billing:3.0</cbc:CustomizationID>
  <cbc:ProfileID>urn:fdc:peppol.eu:2017:poacc:billing:01:1.0</cbc:ProfileID>
  <cbc:ID>{}</cbc:ID>
  <cbc:IssueDate>{}</cbc:IssueDate>
  <cbc:DueDate>{}</cbc:DueDate>
  <cbc:InvoiceTypeCode>380</cbc:InvoiceTypeCode>
  <cbc:DocumentCurrencyCode>{}</cbc:DocumentCurrencyCode>
  <cac:AccountingSupplierParty>
    <cac:Party>
      <cac:PartyName><cbc:Name>{}</cbc:Name></cac:PartyName>
      <cac:PartyTaxScheme><cac:TaxScheme><cbc:ID>VAT</cbc:ID></cac:TaxScheme>{}</cac:PartyTaxScheme>
    </cac:Party>
  </cac:AccountingSupplierParty>
  <cac:AccountingCustomerParty>
    <cac:Party>
      <cac:PartyName><cbc:Name>{}</cbc:Name></cac:PartyName>
      <cac:PartyTaxScheme><cac:TaxScheme><cbc:ID>VAT</cbc:ID></cac:TaxScheme>{}</cac:PartyTaxScheme>
    </cac:Party>
  </cac:AccountingCustomerParty>
  <cac:LegalMonetaryTotal>
    <cbc:PayableAmount currencyID="{}">{:.2}</cbc:PayableAmount>
  </cac:LegalMonetaryTotal>
{}
</Invoice>"#,
        xml_escape(&d.invoice_number),
        xml_escape(&d.invoice_date),
        xml_escape(&d.due_date),
        xml_escape(&d.currency),
        xml_escape(&d.seller_name),
        seller_tax,
        xml_escape(&d.buyer_name),
        buyer_tax,
        xml_escape(&d.currency),
        total,
        lines_xml.trim_end(),
    )
}

// ─── Repo ─────────────────────────────────────────────────────────────────────

pub struct EInvoiceRepo;

impl EInvoiceRepo {
    pub async fn send(
        pool: &PgPool,
        org_id: &str,
        invoice_id: &str,
        input: SendEInvoice,
    ) -> Result<EInvoiceTransmission, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let inv_uuid = parse_uuid(invoice_id)?;

        // Fetch invoice with contact info for XML generation.
        #[derive(sqlx::FromRow)]
        struct InvRow {
            invoice_number: String,
            date: time::Date,
            due_date: time::Date,
            currency: String,
            contact_name: String,
            contact_tax_number: Option<String>,
        }

        let inv: InvRow = sqlx::query_as(
            "SELECT i.invoice_number, i.date, i.due_date, i.currency, \
                    c.name AS contact_name, c.tax_number AS contact_tax_number \
             FROM invoices i \
             JOIN contacts c ON c.id = i.contact_id \
             WHERE i.id = $1 AND i.organization_id = $2 AND i.invoice_type = 'invoice'",
        )
        .bind(inv_uuid)
        .bind(org_uuid)
        .fetch_optional(pool)
        .await
        .map_err(map_sqlx_err)?
        .ok_or(DbError::NotFound)?;

        // Fetch seller (org) name.
        let (org_name,): (String,) = sqlx::query_as("SELECT name FROM organizations WHERE id = $1")
            .bind(org_uuid)
            .fetch_one(pool)
            .await
            .map_err(map_sqlx_err)?;

        // Fetch lines.
        #[derive(sqlx::FromRow)]
        struct LineRow {
            description: String,
            quantity: i64,
            unit_price: i64,
            tax_rate: i64,
        }

        let line_rows: Vec<LineRow> = sqlx::query_as(
            "SELECT description, quantity, unit_price, tax_rate \
             FROM invoice_lines WHERE invoice_id = $1 ORDER BY sort_order ASC",
        )
        .bind(inv_uuid)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;

        let total: i64 = line_rows
            .iter()
            .map(|l| {
                let gross = l.quantity * l.unit_price / 100;
                let tax = gross * l.tax_rate / 10000;
                gross + tax
            })
            .sum();

        let fmt = time::macros::format_description!("[year]-[month]-[day]");
        let lines: Vec<LineData> = line_rows
            .iter()
            .enumerate()
            .map(|(i, l)| LineData {
                id: i + 1,
                description: l.description.clone(),
                quantity: l.quantity,
                unit_price: l.unit_price,
                tax_rate: l.tax_rate,
            })
            .collect();

        let d = InvoiceData {
            invoice_number: inv.invoice_number.clone(),
            invoice_date: inv.date.format(fmt).unwrap_or_default(),
            due_date: inv.due_date.format(fmt).unwrap_or_default(),
            currency: inv.currency.clone(),
            seller_name: org_name,
            seller_tax_number: None,
            buyer_name: inv.contact_name,
            buyer_tax_number: inv.contact_tax_number,
            lines,
            total_amount: total,
        };

        let format = input.format.as_deref().unwrap_or("ubl");
        let xml = build_ubl_xml(&d);

        let row: TransmissionRow = sqlx::query_as(
            "INSERT INTO einvoice_transmissions \
             (organization_id, invoice_id, format, status, transmission_xml, sent_at) \
             VALUES ($1, $2, $3::einvoice_format, 'sent', $4, NOW()) \
             RETURNING id, organization_id, invoice_id, format::TEXT, status::TEXT, \
                       external_id, transmission_xml, error_message, sent_at, \
                       created_at, updated_at",
        )
        .bind(org_uuid)
        .bind(inv_uuid)
        .bind(format)
        .bind(&xml)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;

        Ok(row.into())
    }

    pub async fn get_status(
        pool: &PgPool,
        org_id: &str,
        invoice_id: &str,
    ) -> Result<Vec<EInvoiceTransmission>, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let inv_uuid = parse_uuid(invoice_id)?;

        let rows: Vec<TransmissionRow> = sqlx::query_as(
            "SELECT id, organization_id, invoice_id, format::TEXT, status::TEXT, \
                    external_id, transmission_xml, error_message, sent_at, \
                    created_at, updated_at \
             FROM einvoice_transmissions \
             WHERE invoice_id = $1 AND organization_id = $2 \
             ORDER BY created_at DESC",
        )
        .bind(inv_uuid)
        .bind(org_uuid)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;

        Ok(rows.into_iter().map(EInvoiceTransmission::from).collect())
    }

    pub async fn receive(
        pool: &PgPool,
        org_id: &str,
        input: InboundEInvoice,
    ) -> Result<serde_json::Value, DbError> {
        // Parse minimal fields from UBL XML using simple string extraction.
        let xml = &input.xml;

        // Match opening tags with or without attributes (e.g. <cbc:ID schemeID="...">).
        let extract = |tag: &str| -> Option<String> {
            let tag_prefix = format!("<cbc:{tag}");
            let close = format!("</cbc:{tag}>");
            let tag_start = xml.find(&tag_prefix)?;
            // Advance past any attributes to the closing `>` of the start tag.
            let gt_offset = xml[tag_start..].find('>')?;
            let content_start = tag_start + gt_offset + 1;
            let end_offset = xml[content_start..].find(&close)?;
            Some(xml[content_start..content_start + end_offset].to_string())
        };

        let invoice_number =
            extract("ID").unwrap_or_else(|| format!("UBL-{}", &Uuid::new_v4().to_string()[..8]));
        let issue_date_str = extract("IssueDate").unwrap_or_else(|| {
            let now = OffsetDateTime::now_utc();
            format!("{}-{:02}-{:02}", now.year(), now.month() as u8, now.day())
        });
        let due_date_str = extract("DueDate").unwrap_or_else(|| issue_date_str.clone());
        let currency = extract("DocumentCurrencyCode").unwrap_or_else(|| "USD".to_string());

        let fmt = time::macros::format_description!("[year]-[month]-[day]");
        let bill_date = time::Date::parse(&issue_date_str, fmt)
            .unwrap_or_else(|_| OffsetDateTime::now_utc().date());
        let due_date = time::Date::parse(&due_date_str, fmt).unwrap_or(bill_date);

        let org_uuid = parse_uuid(org_id)?;

        // Find or create a contact from the supplier name.
        let supplier_name_tag = xml
            .find("<cac:AccountingSupplierParty>")
            .and_then(|start| {
                let slice = &xml[start..];
                let name_open = "<cbc:Name>";
                let name_close = "</cbc:Name>";
                let ns = slice.find(name_open)?;
                let ne = slice.find(name_close)?;
                Some(slice[ns + name_open.len()..ne].to_string())
            })
            .unwrap_or_else(|| "Unknown Vendor".to_string());

        let contact_id: Uuid = {
            let existing: Option<(Uuid,)> = sqlx::query_as(
                "SELECT id FROM contacts WHERE organization_id = $1 AND name = $2 LIMIT 1",
            )
            .bind(org_uuid)
            .bind(&supplier_name_tag)
            .fetch_optional(pool)
            .await
            .map_err(map_sqlx_err)?;

            if let Some((id,)) = existing {
                id
            } else {
                let new_id = Uuid::new_v4();
                sqlx::query(
                    "INSERT INTO contacts (id, organization_id, name, contact_type) \
                     VALUES ($1, $2, $3, 'vendor')",
                )
                .bind(new_id)
                .bind(org_uuid)
                .bind(&supplier_name_tag)
                .execute(pool)
                .await
                .map_err(map_sqlx_err)?;
                new_id
            }
        };

        let bill = BillRepo::create(
            pool,
            org_id,
            CreateVendorBill {
                contact_id: Some(contact_id.to_string()),
                bill_date,
                due_date: Some(due_date),
                reference: Some(invoice_number.clone()),
                description: format!("Received via e-invoice (UBL). Original ID: {invoice_number}"),
                currency_code: currency,
                exchange_rate: rust_decimal::Decimal::ONE,
                lines: vec![CreateBillLine {
                    account_id: None,
                    description: Some("Imported from e-invoice".to_string()),
                    quantity: 1,
                    unit_price: 0,
                    tax_rate: 0,
                    variant_id: None,
                }],
                purchase_order_id: None,
            },
        )
        .await?;

        Ok(serde_json::json!({
            "bill_id": bill.id,
            "invoice_number": invoice_number,
            "status": "imported",
        }))
    }
}

pub mod account;
pub mod api_key;
pub mod attachment;
pub mod bank;
pub mod bank_rule;
pub mod batch_payment;
pub mod budget;
pub mod client_portal;
pub mod closed_period;
pub mod consolidated;
pub mod contact;
pub mod custom_field;
pub mod department;
pub mod dunning;
pub mod email;
pub mod expense;
pub mod expense_policy;
pub mod fixed_asset;
pub mod fx;
pub mod identity;
pub mod import;
pub mod inventory;
pub mod invoice;
pub mod invoice_template;
pub mod late_fee;
pub mod mileage;
pub mod note;
pub mod notification;
pub mod organization;
pub mod payment;
pub mod payment_link;
pub mod payroll;
pub mod price_list;
pub mod product;
pub mod project;
pub mod purchase_order;
pub mod recurring;
pub mod reports;
pub mod retainer;
pub mod role;
pub mod session;
pub mod tag;
pub mod tax_rate;
pub mod time_entry;
pub mod transaction;
pub mod webhook;

pub use account::{Account, AccountType, CreateAccount, UpdateAccount};
pub use api_key::{ApiKey, CreateApiKey, CreatedApiKey};
pub use attachment::{Attachment, CreateAttachment};
pub use bank::{
    BankAccount, BankTransaction, CreateBankAccount, ImportBankTransaction, MatchTransaction,
    ReconciliationSummary, UpdateBankAccount,
};
pub use bank_rule::{BankRule, CreateBankRule};
pub use batch_payment::{BatchPayment, BatchPaymentLine, CreateBatchPayment};
pub use budget::{
    Budget, BudgetLine, BudgetVsActualLine, BudgetVsActualReport, CreateBudget, UpdateBudget,
    UpsertBudgetLine,
};
pub use client_portal::{ClientPortalToken, CreateClientPortalToken};
pub use closed_period::{ClosedPeriod, CreateClosedPeriod};
pub use consolidated::{ConsolidatedProfitLoss, OrgProfitLoss};
pub use contact::{Contact, ContactType, CreateContact, UpdateContact};
pub use custom_field::{
    CreateCustomFieldDefinition, CustomFieldDefinition, CustomFieldValue, SetCustomFieldValue,
    UpdateCustomFieldDefinition,
};
pub use department::{CreateDepartment, Department, DepartmentPlReport, UpdateDepartment};
pub use dunning::{CreateDunningRule, DunningRule, InvoiceReminder, OverdueInvoice};
pub use email::{EmailLog, EmailSettings, SendEmailRequest, UpsertEmailSettings};
pub use expense::{CreateExpense, Expense, ExpenseStatus, UpdateExpense};
pub use expense_policy::{ContactStatement, ExpensePolicy, StatementLine, UpsertExpensePolicy};
pub use fixed_asset::{
    AssetRegisterRow, CreateFixedAsset, DepreciationMethod, FixedAsset, UpdateFixedAsset,
};
pub use fx::{FxSummaryRow, RealizedFxEntry};
pub use identity::{
    CreateOidcProvider, CreateSamlProvider, CreateScimToken, CreatedScimToken, IdentityProvider,
    ProviderSummary, ProviderType, ScimToken,
};
pub use import::{AccountCsvRow, ContactCsvRow, ImportError, ImportResult};
pub use inventory::{
    CreateInventoryItem, InventoryAdjustment, InventoryItem, InventoryMovement, LowStockItem,
    UpdateInventoryItem,
};
pub use invoice::{
    CreateInvoice, CreateInvoiceLine, Invoice, InvoiceFilters, InvoiceLine, InvoiceStatus,
    InvoiceType, UpdateInvoice,
};
pub use invoice_template::{InvoiceTemplate, UpsertInvoiceTemplate};
pub use late_fee::{LateFee, LateFeeRule, UpsertLateFeeRule};
pub use mileage::{CreateMileageTrip, MileageSummary, MileageTrip};
pub use note::{CreateNote, Note};
pub use notification::{CreateNotification, Notification};
pub use organization::{CreateOrganization, Organization, UpdateOrganization};
pub use payment::{CreatePayment, Payment, VALID_METHODS};
pub use payment_link::{CreatePaymentLink, PaymentLink};
pub use payroll::{
    CreatePayrollEntry, CreatePayrollRun, PayrollEntry, PayrollRun, PayrollRunSummary,
};
pub use price_list::{
    CreatePriceList, PriceList, PriceListItem, SpendAnalysisReport, SpendAnalysisRow,
    UpsertPriceListItem,
};
pub use product::{CreateProduct, Product, UpdateProduct};
pub use project::{CreateProject, Project, ProjectSummary, UpdateProject};
pub use purchase_order::{
    CreatePoLine, CreatePurchaseOrder, PoStatus, PurchaseOrder, PurchaseOrderLine, ReceivePoLine,
    UpdatePurchaseOrder,
};
pub use recurring::{
    CreateRecurringSchedule, Frequency, RecurringSchedule, UpdateRecurringSchedule,
};
pub use reports::{
    AccountBalance, AgingReport, AgingRow, BalanceSheetReport, CashFlowReport, CashFlowSection,
    DashboardKpis, ProfitLossReport, ReportLine, ReportSection, SearchHit, Summary1099,
    TaxSummaryLine, TaxSummaryReport, TrialBalance, Vendor1099Row,
};
pub use retainer::{ApplyRetainer, CreateRetainer, DepositRetainer, Retainer, RetainerTransaction};
pub use role::{AssignPermission, CreateRole, Permission, Role};
pub use session::Session;
pub use tag::{CreateTag, Tag, UpdateTag};
pub use tax_rate::{CreateTaxRate, TaxRate, UpdateTaxRate};
pub use time_entry::{
    BillTimeEntries, CreateTimeEntry, TimeEntry, TimeSummaryRow, UpdateTimeEntry,
};
pub use transaction::{
    CreateJournalEntry, CreateJournalLine, JournalEntry, JournalEntryStatus, JournalLine,
};
pub use webhook::{
    CreateWebhookEndpoint, UpdateWebhookEndpoint, WebhookEndpoint, WebhookPayload, ALL_EVENT_TYPES,
};

/// Serde helpers for `Option<time::Date>` as `"YYYY-MM-DD"`.
pub mod opt_date_serde {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use time::{format_description, Date};

    pub fn serialize<S: Serializer>(date: &Option<Date>, s: S) -> Result<S::Ok, S::Error> {
        match date {
            Some(d) => {
                let fmt = format_description::parse("[year]-[month]-[day]")
                    .expect("static format is valid");
                s.serialize_some(&d.format(&fmt).map_err(serde::ser::Error::custom)?)
            }
            None => Option::<String>::None.serialize(s),
        }
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Option<Date>, D::Error> {
        let raw: Option<String> = Option::deserialize(d)?;
        match raw {
            None => Ok(None),
            Some(s) => {
                let fmt = format_description::parse("[year]-[month]-[day]")
                    .expect("static format is valid");
                Date::parse(&s, &fmt)
                    .map(Some)
                    .map_err(serde::de::Error::custom)
            }
        }
    }
}

/// Serde helpers for `time::Date` as `"YYYY-MM-DD"`.
pub mod date_serde {
    use serde::{Deserialize, Deserializer, Serializer};
    use time::{format_description, Date};

    pub fn serialize<S: Serializer>(date: &Date, s: S) -> Result<S::Ok, S::Error> {
        let fmt =
            format_description::parse("[year]-[month]-[day]").expect("static format is valid");
        s.serialize_str(&date.format(&fmt).map_err(serde::ser::Error::custom)?)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Date, D::Error> {
        let raw = String::deserialize(d)?;
        let fmt =
            format_description::parse("[year]-[month]-[day]").expect("static format is valid");
        Date::parse(&raw, &fmt).map_err(serde::de::Error::custom)
    }
}

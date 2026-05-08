pub mod account;
pub mod ach;
pub mod api_key;
pub mod approval_chain;
pub mod approval_rule;
pub mod assembly_order;
pub mod attachment;
pub mod bank;
pub mod bank_deposit;
pub mod bank_feed;
pub mod bank_reconciliation;
pub mod bank_rule;
pub mod batch_payment;
pub mod bill;
pub mod budget;
pub mod cash_sale;
pub mod check_run;
pub mod client_portal;
pub mod closed_period;
pub mod commission;
pub mod consolidated;
pub mod consolidation_elimination;
pub mod contact;
pub mod contact_group;
pub mod contractor_tax_info;
pub mod cost_code;
pub mod credit_note;
pub mod custom_field;
pub mod deferred_charge;
pub mod deferred_revenue;
pub mod department;
pub mod direct_deposit;
pub mod doc_sequence;
pub mod dunning;
pub mod einvoice;
pub mod email;
pub mod employee;
pub mod employee_bank_account;
pub mod employee_loan;
pub mod expense;
pub mod expense_claim;
pub mod expense_policy;
pub mod expense_report;
pub mod fixed_asset;
pub mod fx;
pub mod fx_revaluation;
pub mod grn;
pub mod identity;
pub mod import;
pub mod intercompany;
pub mod inventory;
pub mod inventory_lot;
pub mod inventory_reorder_request;
pub mod inventory_serial_number;
pub mod inventory_stocktake;
pub mod invoice;
pub mod invoice_template;
pub mod landed_cost;
pub mod late_fee;
pub mod leave;
pub mod mileage;
pub mod note;
pub mod notification;
pub mod organization;
pub mod payment;
pub mod payment_link;
pub mod payment_plan;
pub mod payment_terms;
pub mod payroll;
pub mod payroll_tax;
pub mod payslip;
pub mod plaid;
pub mod prepaid_expense;
pub mod prepayment;
pub mod price_list;
pub mod product;
pub mod product_category;
pub mod product_variant;
pub mod progress_claim;
pub mod project;
pub mod project_phase;
pub mod project_task;
pub mod purchase_order;
pub mod purchase_requisition;
pub mod purchase_return;
pub mod quote;
pub mod recurring;
pub mod recurring_bill;
pub mod recurring_invoice;
pub mod recurring_journal_entry;
pub mod report_schedule;
pub mod reports;
pub mod retainer;
pub mod rev_rec;
pub mod role;
pub mod sales_order;
pub mod sales_order_shipment;
pub mod sales_return;
pub mod sales_tax_nexus;
pub mod service_territory;
pub mod session;
pub mod subscription;
pub mod tag;
pub mod tax_filing;
pub mod tax_group;
pub mod tax_period;
pub mod tax_rate;
pub mod tax_rule;
pub mod time_entry;
pub mod tracking_category;
pub mod transaction;
pub mod vendor_credit;
pub mod vendor_portal;
pub mod warehouse;
pub mod webhook;
pub mod work_order;

pub use account::{Account, AccountType, CreateAccount, UpdateAccount};
pub use ach::{AchPayment, CollectAch, GenerateNachaRequest, NachaFile, PayBillAch};
pub use api_key::{ApiKey, CreateApiKey, CreatedApiKey};
pub use approval_chain::{
    ApprovalChain, ApprovalChainStep, ApprovalDecision, ApprovalRequest, CreateApprovalChain,
    CreateApprovalChainStep, RecordApprovalDecision, SubmitApprovalRequest,
};
pub use approval_rule::{ApprovalRule, CreateApprovalRule, UpdateApprovalRule};
pub use assembly_order::{
    AssemblyOrder, AssemblyOrderLine, CreateAssemblyOrder, CreateAssemblyOrderLine,
};
pub use attachment::{Attachment, CreateAttachment};
pub use bank::{
    BankAccount, BankTransaction, CreateBankAccount, ImportBankTransaction, MatchTransaction,
    ReconciliationSummary, UpdateBankAccount,
};
pub use bank_deposit::{
    BankDeposit, BankDepositItem, ClearBankDeposit, CreateBankDeposit, CreateDepositItem,
};
pub use bank_feed::{
    BankFeedAutoMatchResult, BankFeedTransaction, ImportBankFeed, ImportBankFeedRow,
    MatchBankFeedTransaction,
};
pub use bank_reconciliation::{BankReconciliationStatement, CreateBankReconciliationStatement};
pub use bank_rule::{BankRule, CreateBankRule};
pub use batch_payment::{BatchPayment, BatchPaymentLine, CreateBatchPayment};
pub use bill::{
    BillLine, BillPayment, CreateBillLine, CreateBillPayment, CreateSpendMoney, CreateVendorBill,
    UpdateVendorBill, VendorBill,
};
pub use budget::{
    Budget, BudgetLine, BudgetVsActualLine, BudgetVsActualReport, CreateBudget, UpdateBudget,
    UpsertBudgetLine,
};
pub use cash_sale::CreateCashSale;
pub use check_run::{CheckRun, CheckRunItem, CreateCheckRun, CreateCheckRunItem};
pub use client_portal::{
    ClientPortalToken, CreateClientPortalToken, CreatePortalAutopay, CreatePortalPaymentMethod,
    PortalAutopayEnrollment, PortalPayInvoice, PortalPaymentMethod,
};
pub use closed_period::{ClosedPeriod, CreateClosedPeriod};
pub use commission::{CreateSalesCommission, PayCommission, SalesCommission};
pub use consolidated::{ConsolidatedProfitLoss, OrgProfitLoss};
pub use consolidation_elimination::{ConsolidationElimination, CreateConsolidationElimination};
pub use contact::{Contact, ContactCreditStatus, ContactType, CreateContact, UpdateContact};
pub use contact_group::{ContactGroup, CreateContactGroup, UpdateContactGroup};
pub use contractor_tax_info::{
    Contractor1099Summary, ContractorTaxInfo, CreateContractorTaxInfo, UpdateContractorTaxInfo,
};
pub use cost_code::{CostCode, CreateCostCode, UpdateCostCode};
pub use credit_note::{ApplyCreditNote, CreateCreditNote, CreditNote, CreditNoteApplication};
pub use custom_field::{
    CreateCustomFieldDefinition, CustomFieldDefinition, CustomFieldValue, SetCustomFieldValue,
    UpdateCustomFieldDefinition,
};
pub use deferred_charge::{
    CreateDeferredCharge, DeferredCharge, InvoiceDeferredCharges, ProgressInvoiceInput,
    UpdateDeferredCharge,
};
pub use deferred_revenue::{
    CreateDeferredRevenueSchedule, DeferredRevenueEntry, DeferredRevenueSchedule, RecognizeRevenue,
};
pub use department::{CreateDepartment, Department, DepartmentPlReport, UpdateDepartment};
pub use direct_deposit::{
    CreateDirectDepositBatch, CreateDirectDepositEntry, DirectDepositBatch, DirectDepositEntry,
    MarkBatchSent,
};
pub use doc_sequence::{DocSequence, ResetDocSequence, UpsertDocSequence};
pub use dunning::{CreateDunningRule, DunningRule, InvoiceReminder, OverdueInvoice};
pub use einvoice::{EInvoiceTransmission, InboundEInvoice, SendEInvoice};
pub use email::{EmailLog, EmailSettings, SendEmailRequest, UpsertEmailSettings};
pub use employee::{CreateEmployee, Employee, UpdateEmployee};
pub use employee_bank_account::{
    CreateEmployeeBankAccount, EmployeeBankAccount, UpdateEmployeeBankAccount,
};
pub use employee_loan::{
    CreateEmployeeLoan, CreateLoanRepayment, EmployeeLoan, LoanRepayment, UpdateEmployeeLoan,
};
pub use expense::{
    BillableExpenseRef, CreateExpense, CreateExpenseCategory, Expense, ExpenseCategory,
    ExpenseStatus, UpdateExpense, UpdateExpenseCategory,
};
pub use expense_claim::{
    CreateExpenseClaim, CreateExpenseClaimLine, ExpenseClaim, ExpenseClaimLine, ReviewExpenseClaim,
    UpdateExpenseClaim,
};
pub use expense_policy::{ContactStatement, ExpensePolicy, StatementLine, UpsertExpensePolicy};
pub use expense_report::{
    AddExpenseToReport, CreateExpenseReport, ExpenseReport, UpdateExpenseReport,
};
pub use fixed_asset::{
    AssetRegisterRow, BulkDepreciationResult, CreateFixedAsset, DepreciationMethod,
    DepreciationScheduleLine, FixedAsset, UpdateFixedAsset,
};
pub use fx::{FxSummaryRow, RealizedFxEntry};
pub use fx_revaluation::{CreateFxRevaluation, FxRevaluation};
pub use grn::{CreateGrn, CreateGrnLine, GoodsReceiptNote, GrnLine};
pub use identity::{
    CreateOidcProvider, CreateSamlProvider, CreateScimToken, CreatedScimToken, IdentityProvider,
    ProviderSummary, ProviderType, ScimToken,
};
pub use import::{AccountCsvRow, ContactCsvRow, ImportError, ImportResult};
pub use intercompany::{
    CreateIntercompanyLink, CreateIntercompanyTransaction, IntercompanyLink,
    IntercompanyTransaction,
};
pub use inventory::{
    CreateInventoryItem, InventoryAdjustment, InventoryAvailability, InventoryItem,
    InventoryMovement, InventoryValuationReport, InventoryValuationRow, LowStockItem,
    UpdateInventoryItem,
};
pub use inventory_lot::{CreateInventoryLot, InventoryLot, UpdateInventoryLot};
pub use inventory_reorder_request::{
    CreateInventoryReorderRequest, InventoryReorderRequest, SubmitInventoryReorderRequest,
};
pub use inventory_serial_number::{
    CreateInventorySerialNumber, InventorySerialNumber, UpdateInventorySerialNumber,
};
pub use inventory_stocktake::{
    CreateInventoryStocktake, InventoryStocktake, InventoryStocktakeLine, UpdateStocktakeLine,
};
pub use invoice::{
    CreateInvoice, CreateInvoiceLine, Invoice, InvoiceFilters, InvoiceLine, InvoiceStatus,
    InvoiceType, UpdateInvoice,
};
pub use invoice_template::{InvoiceTemplate, UpsertInvoiceTemplate};
pub use landed_cost::{CreateLandedCost, LandedCost, LandedCostAllocation};
pub use late_fee::{LateFee, LateFeeRule, UpsertLateFeeRule};
pub use leave::{
    CreateLeaveRequest, CreateLeaveType, LeaveBalance, LeaveRequest, LeaveType, UpdateLeaveType,
};
pub use mileage::{CreateMileageTrip, MileageSummary, MileageTrip};
pub use note::{CreateNote, Note};
pub use notification::{CreateNotification, Notification};
pub use organization::{CreateOrganization, Organization, UpdateOrganization};
pub use payment::{CreatePayment, CreateRefund, Payment, Refund, VALID_METHODS};
pub use payment_link::{CreatePaymentLink, PaymentLink};
pub use payment_plan::{
    CreateInstallment, CreatePaymentPlan, PayInstallment, PaymentPlan, PaymentPlanInstallment,
};
pub use payment_terms::{CreatePaymentTerms, PaymentTerms, UpdatePaymentTerms};
pub use payroll::{
    CreatePayrollEntry, CreatePayrollRun, PayrollEntry, PayrollRun, PayrollRunSummary,
};
pub use payroll_tax::{CreatePayrollTaxLiability, PayPayrollTax, PayrollTaxLiability};
pub use payslip::{CreatePayslip, Payslip};
pub use plaid::{ExchangePlaidToken, PlaidItem, PlaidSyncRequest, PlaidSyncResult};
pub use prepaid_expense::{
    CreatePrepaidExpenseSchedule, PrepaidExpenseEntry, PrepaidExpenseSchedule,
    UpdatePrepaidExpenseSchedule,
};
pub use prepayment::{ApplyPrepayment, CreatePrepayment, Prepayment};
pub use price_list::{
    CreatePriceList, PriceList, PriceListItem, SpendAnalysisReport, SpendAnalysisRow,
    UpsertPriceListItem,
};
pub use product::{
    BundleComponent, BundleComponentInput, CreateProduct, Product, SetBundleComponents,
    UpdateProduct,
};
pub use product_category::{CreateProductCategory, ProductCategory, UpdateProductCategory};
pub use product_variant::{CreateProductVariant, ProductVariant, UpdateProductVariant};
pub use progress_claim::{
    CreateProgressClaim, ProgressClaim, ProjectBillingReport, ProjectBillingRow, ReleaseRetainage,
};
pub use project::{CreateProject, Project, ProjectSummary, UpdateProject};
pub use project_phase::{CreateProjectPhase, ProjectPhase, UpdateProjectPhase};
pub use project_task::{CreateProjectTask, ProjectTask, UpdateProjectTask};
pub use purchase_order::{
    CreatePoLine, CreatePurchaseOrder, PoStatus, PurchaseOrder, PurchaseOrderLine, ReceivePoLine,
    UpdatePurchaseOrder,
};
pub use purchase_requisition::{
    ConvertPrToPo, CreatePrLine, CreatePurchaseRequisition, PrLine, PurchaseRequisition,
    UpdatePurchaseRequisition,
};
pub use purchase_return::{
    ApprovePurchaseReturn, CreatePurchaseReturn, CreatePurchaseReturnLine, PurchaseReturn,
    PurchaseReturnLine, ShipPurchaseReturn,
};
pub use quote::{
    ConvertQuoteToInvoice, CreateQuote, CreateQuoteLine, Quote, QuoteLine, UpdateQuote,
};
pub use recurring::{
    CreateRecurringSchedule, Frequency, RecurringSchedule, UpdateRecurringSchedule,
};
pub use recurring_bill::{
    CreateRecurringBill, CreateRecurringBillLine, RecurringBill, RecurringBillLine,
    UpdateRecurringBill,
};
pub use recurring_invoice::{
    CreateRecurringInvoice, CreateRecurringInvoiceLine, RecurringInvoice, RecurringInvoiceLine,
    UpdateRecurringInvoice,
};
pub use recurring_journal_entry::{
    CreateRecurringJournalEntry, CreateRecurringJournalEntryLine, RecurringJournalEntry,
    RecurringJournalEntryLine, UpdateRecurringJournalEntry,
};
pub use report_schedule::{CreateReportSchedule, ReportSchedule, UpdateReportSchedule};
pub use reports::{
    AccountBalance, AccountLedger, AgingReport, AgingRow, ApAgingDetailReport, ApAgingDetailRow,
    ArAgingDetailReport, ArAgingDetailRow, AutoReversalResult, BalanceSheetComparisonReport,
    BalanceSheetComparisonSection, BalanceSheetReport, CashBasisBalanceSheet,
    CashDisbursementsJournal, CashDisbursementsJournalRow, CashFlowForecast,
    CashFlowForecastBucket, CashFlowIndirectLine, CashFlowIndirectReport, CashFlowIndirectSection,
    CashFlowReport, CashFlowSection, CashReceiptsJournal, CashReceiptsJournalRow,
    ContactBalanceRow, CurrencyExposureReport, CurrencyExposureRow, CustomerBalancesReport,
    DashboardKpis, EquityStatement, EquityStatementLine, Form941Quarter, GrniReport, GrniRow,
    InventoryAgingReport, InventoryAgingRow, JobCostingCostCodeRow, JobCostingReport,
    JobCostingRow, LedgerLine, OutstandingQuoteRow, OutstandingQuotesReport, PLComparisonReport,
    PayrollSummaryReport, PayrollSummaryRow, PoSpendingReport, PoSpendingRow, ProfitLossReport,
    ProjectBurnReport, ProjectBurnRow, ProjectProfitabilityReport, ProjectProfitabilityRow,
    RemittanceAdvice, RemittanceLine, ReportLine, ReportSection, SalesByCustomerReport,
    SalesByCustomerRow, SalesByProductReport, SalesByProductRow, SalesByRepReport, SalesByRepRow,
    SalesTaxByNexusReport, SalesTaxByNexusRow, SearchHit, Summary1099, TaxSummaryLine,
    TaxSummaryReport, TrackingPLReport, TrackingPLRow, TrialBalance, VatReturnLine,
    VatReturnReport, Vendor1099Row, VendorBalancesReport, VendorSpendReport, VendorSpendRow, W2Row,
};
pub use retainer::{ApplyRetainer, CreateRetainer, DepositRetainer, Retainer, RetainerTransaction};
pub use rev_rec::{CreateRevRecSchedule, RecognizeRevRec, RevRecEntry, RevRecSchedule};
pub use role::{AssignPermission, CreateRole, Permission, Role};
pub use sales_order::{
    ConvertSoToInvoice, CreateSalesOrder, CreateSoLine, SalesOrder, SoLine, UpdateSalesOrder,
};
pub use sales_order_shipment::{
    CreateSalesOrderShipment, CreateShipmentLine, SalesOrderShipment, ShipmentLine,
};
pub use sales_return::{
    ApproveSalesReturn, CreateSalesReturn, CreateSalesReturnLine, ReceiveSalesReturn, SalesReturn,
    SalesReturnLine,
};
pub use sales_tax_nexus::{CreateSalesTaxNexus, SalesTaxNexus, UpdateSalesTaxNexus};
pub use service_territory::{CreateServiceTerritory, ServiceTerritory, UpdateServiceTerritory};
pub use session::Session;
pub use subscription::{
    CreateSubscription, CreateSubscriptionPlan, Subscription, SubscriptionPlan, UpdateSubscription,
    UpdateSubscriptionPlan,
};
pub use tag::{CreateTag, Tag, UpdateTag};
pub use tax_filing::{HstGstReturn, T4AFilingSummary, T4ASlip, T4Slip, T4Summary, TaxFiling};
pub use tax_group::{CreateTaxGroup, TaxGroup, TaxGroupRate, TaxGroupRateInput, UpdateTaxGroup};
pub use tax_period::{CreateTaxPeriod, FileTaxPeriod, TaxPeriod};
pub use tax_rate::{CreateTaxRate, TaxRate, UpdateTaxRate};
pub use tax_rule::{CreateTaxRule, SuggestedTaxRate, TaxRule, UpdateTaxRule};
pub use time_entry::{
    BillTimeEntries, BulkApproveTimeEntries, BulkRejectTimeEntries, CreateTimeEntry,
    RejectTimeEntry, TimeEntry, TimeSummaryRow, UpdateTimeEntry,
};
pub use tracking_category::{
    CreateTrackingCategory, CreateTrackingOption, TrackingCategory, TrackingOption,
    UpdateTrackingCategory, UpdateTrackingOption,
};
pub use transaction::{
    CreateJournalEntry, CreateJournalLine, JournalEntry, JournalEntryStatus, JournalLine,
};
pub use vendor_credit::{
    ApplyVendorCredit, CreateVendorCredit, CreateVendorCreditLine, VendorCredit,
    VendorCreditApplication, VendorCreditLine,
};
pub use vendor_portal::{CreateVendorPortalToken, VendorPortalToken};
pub use warehouse::{
    CreatePendingTransfer, CreateStockAdjustment, CreateWarehouse, InventoryTransfer,
    StockAdjustment, StockSummaryRow, TransferStock, UpdateWarehouse, Warehouse, WarehouseStock,
    WarehouseStockLine,
};
pub use webhook::{
    CreateWebhookEndpoint, UpdateWebhookEndpoint, WebhookEndpoint, WebhookPayload, ALL_EVENT_TYPES,
};
pub use work_order::{
    CreateWorkOrder, CreateWorkOrderLine, UpdateWorkOrder, WorkOrder, WorkOrderLine,
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

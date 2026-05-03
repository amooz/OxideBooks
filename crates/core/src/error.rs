use thiserror::Error;

#[derive(Debug, Error)]
pub enum CoreError {
    #[error("journal entry is not balanced: debits {debits} != credits {credits}")]
    UnbalancedEntry { debits: i64, credits: i64 },

    #[error("journal entry must have at least two lines")]
    InsufficientLines,

    #[error("a journal line cannot have both a debit and a credit amount")]
    BothDebitAndCredit,

    #[error("unknown account type: {0}")]
    UnknownAccountType(String),

    #[error("unknown invoice status: {0}")]
    UnknownInvoiceStatus(String),

    #[error("amount must be non-negative, got {0}")]
    NegativeAmount(i64),

    #[error("invoice line quantity must be positive")]
    ZeroQuantity,
}

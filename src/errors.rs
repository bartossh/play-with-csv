use rust_decimal::Decimal;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum LedgerError {
    #[error("insufficient funds: tried to withdraw {amount}, only {balance} available")]
    InsufficientFunds { amount: Decimal, balance: Decimal },

    #[error("account {0} is locked")]
    AccountLocked(u16),

    #[error("transaction {0} not found")]
    TxNotFound(u32),

    #[error("transaction {0} is duplicated")]
    TxDuplicated(u32),

    #[error("value overflow")]
    ValueOverflow,
}

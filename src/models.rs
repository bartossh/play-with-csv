use crate::errors::LedgerError;
use anyhow::Result;
use getset::Getters;
use rust_decimal::prelude::*;
use serde::{Deserialize, Serialize};

const ZERO: Decimal = dec!(0.0000);

fn round_four_decimals<S>(x: &Decimal, s: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    s.serialize_str(&format!("{x:.4}"))
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Getters)]
pub struct ClientBalance {
    #[getset(get = "pub")]
    client: u16,
    #[getset(get = "pub")]
    #[serde(serialize_with = "round_four_decimals")]
    available: Decimal,
    #[getset(get = "pub")]
    #[serde(serialize_with = "round_four_decimals")]
    held: Decimal,
    #[getset(get = "pub")]
    #[serde(serialize_with = "round_four_decimals")]
    total: Decimal,
    #[getset(get = "pub")]
    locked: bool,
}

impl ClientBalance {
    pub fn new(client: u16) -> Self {
        ClientBalance {
            client,
            available: dec!(0.0000),
            held: dec!(0.0000),
            total: dec!(0.0000),
            locked: false,
        }
    }

    /// Deposits an amount into the client's balance.
    ///
    /// # Arguments
    /// * `amount` - The amount to deposit.
    ///
    /// # Returns
    /// A Result indicating success or failure.
    pub fn deposit(&mut self, amount: &str) -> Result<()> {
        self.validate_is_unlocked()?;
        let amount = amount.parse::<Decimal>()?;

        self.available = self
            .available
            .checked_add(amount)
            .ok_or(LedgerError::ValueOverflow)?;

        self.total = self
            .total
            .checked_add(amount)
            .ok_or(LedgerError::ValueOverflow)?;
        Ok(())
    }

    pub fn withdraw(&mut self, amount: &str) -> Result<()> {
        self.validate_is_unlocked()?;
        let amount = amount.parse::<Decimal>()?;

        let available =
            self.available
                .checked_sub(amount)
                .ok_or(LedgerError::InsufficientFunds {
                    amount,
                    balance: self.available,
                })?;

        let total = self
            .total
            .checked_sub(amount)
            .ok_or(LedgerError::InsufficientFunds {
                amount,
                balance: self.total,
            })?;

        if total < ZERO {
            return Err(LedgerError::InsufficientFunds {
                amount,
                balance: total,
            })?;
        }
        if available < ZERO {
            return Err(LedgerError::InsufficientFunds {
                amount,
                balance: available,
            })?;
        }

        self.available = available;
        self.total = total;

        Ok(())
    }

    /// Disputes an amount from the client's balance.
    ///
    /// # Arguments
    /// * `amount` - The amount to dispute.
    ///
    /// # Returns
    /// A Result indicating success or failure.
    pub fn dispute(&mut self, amount: &str) -> Result<()> {
        self.validate_is_unlocked()?;
        let amount = amount.parse::<Decimal>()?;
        let available =
            self.available
                .checked_sub(amount)
                .ok_or(LedgerError::InsufficientFunds {
                    amount,
                    balance: self.available,
                })?;

        if available < ZERO {
            return Err(LedgerError::InsufficientFunds {
                amount,
                balance: available,
            })?;
        }

        self.available = available;

        self.held = self
            .held
            .checked_add(amount)
            .ok_or(LedgerError::ValueOverflow)?;
        Ok(())
    }

    /// Resolves a dispute by adding the disputed amount back to the client's available balance.
    ///
    /// # Arguments
    /// * `amount` - The amount to resolve.
    ///
    /// # Returns
    /// A Result indicating success or failure.
    pub fn resolve(&mut self, amount: &str) -> Result<()> {
        self.validate_is_unlocked()?;
        let amount = amount.parse::<Decimal>()?;
        let held = self
            .held
            .checked_sub(amount)
            .ok_or(LedgerError::InsufficientFunds {
                amount,
                balance: self.held,
            })?;

        if held < Decimal::ZERO {
            return Err(LedgerError::InsufficientFunds {
                amount,
                balance: self.held,
            })?;
        }
        self.held = held;

        self.available = self
            .available
            .checked_add(amount)
            .ok_or(LedgerError::ValueOverflow)?;
        Ok(())
    }

    /// Charges back a disputed amount, locking the account.
    ///
    /// # Arguments
    /// * `amount` - The amount to charge back.
    ///
    /// # Returns
    /// A Result indicating success or failure.
    pub fn chargeback(&mut self, amount: &str) -> Result<()> {
        self.validate_is_unlocked()?;
        let amount = amount.parse::<Decimal>()?;
        let held = self
            .held
            .checked_sub(amount)
            .ok_or(LedgerError::InsufficientFunds {
                amount,
                balance: self.held,
            })?;
        let total = self
            .total
            .checked_sub(amount)
            .ok_or(LedgerError::InsufficientFunds {
                amount,
                balance: self.total,
            })?;

        if held < Decimal::ZERO {
            return Err(LedgerError::InsufficientFunds {
                amount,
                balance: self.held,
            })?;
        }
        if total < Decimal::ZERO {
            return Err(LedgerError::InsufficientFunds {
                amount,
                balance: self.total,
            })?;
        }
        self.held = held;
        self.total = total;

        self.locked = true;
        Ok(())
    }

    fn validate_is_unlocked(&self) -> Result<()> {
        if self.locked {
            return Err(LedgerError::AccountLocked(self.client))?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum TransactionType {
    #[serde(rename = "deposit")]
    Deposit,
    #[serde(rename = "withdrawal")]
    Withdrawal,
    #[serde(rename = "dispute")]
    Dispute,
    #[serde(rename = "resolve")]
    Resolve,
    #[serde(rename = "chargeback")]
    Chargeback,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Getters)]
pub struct Transaction {
    #[getset(get = "pub")]
    tx: u32,
    #[getset(get = "pub")]
    client: u16,
    #[getset(get = "pub")]
    amount: String,
    #[getset(get = "pub")]
    #[serde(rename = "type")]
    type_: TransactionType,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deposit_increases_balance() -> Result<()> {
        let mut client = ClientBalance::new(1);
        client.deposit("100.1234")?;
        assert_eq!(client.available, dec!(100.1234));
        assert_eq!(client.total, dec!(100.1234));

        client.deposit("0.1")?;
        assert_eq!(client.available, dec!(100.2234));
        assert_eq!(client.total, dec!(100.2234));

        client.deposit("1.0")?;
        assert_eq!(client.available, dec!(101.2234));
        assert_eq!(client.total, dec!(101.2234));

        client.deposit("1.1")?;
        assert_eq!(client.available, dec!(102.3234));
        assert_eq!(client.total, dec!(102.3234));
        Ok(())
    }

    #[test]
    fn test_withdraw_decreases_balance() -> Result<()> {
        let mut client = ClientBalance::new(1);
        client.deposit("200.5000")?;
        client.withdraw("50.5000")?;
        assert_eq!(client.available, dec!(150.0000));
        assert_eq!(client.total, dec!(150.0000));
        Ok(())
    }

    #[test]
    fn test_withdraw_insufficient_funds() -> Result<()> {
        let mut client = ClientBalance::new(1);
        client.deposit("50")?;
        let res = client.withdraw("100");
        assert!(res.is_err());
        Ok(())
    }

    #[test]
    fn test_dispute_moves_funds_to_held() -> Result<()> {
        let mut client = ClientBalance::new(1);
        client.deposit("150")?;
        client.dispute("50")?;
        assert_eq!(client.available, dec!(100));
        assert_eq!(client.held, dec!(50));
        assert_eq!(client.total, dec!(150));
        Ok(())
    }

    #[test]
    fn test_resolve_returns_held_to_available() -> Result<()> {
        let mut client = ClientBalance::new(1);
        client.deposit("200")?;
        client.dispute("80")?;
        client.resolve("80")?;
        assert_eq!(client.available, dec!(200));
        assert_eq!(client.held, dec!(0));
        assert_eq!(client.total, dec!(200));
        Ok(())
    }

    #[test]
    fn test_cannot_resolve_returns_not_enough_held() -> Result<()> {
        let mut client = ClientBalance::new(1);
        client.deposit("200")?;
        client.dispute("80")?;
        let res = client.resolve("90");
        assert!(res.is_err());
        Ok(())
    }

    #[test]
    fn test_chargeback_locks_account() -> Result<()> {
        let mut client = ClientBalance::new(1);
        client.deposit("120")?;
        client.dispute("50")?;
        client.chargeback("50")?;
        assert_eq!(client.total, dec!(70));
        assert_eq!(client.held, dec!(0));
        assert!(client.locked);
        Ok(())
    }

    #[test]
    fn test_chargeback_not_enough_held_negative_balance() -> Result<()> {
        let mut client = ClientBalance::new(1);
        client.deposit("120")?;
        client.dispute("50")?;
        let res = client.chargeback("51");
        assert!(res.is_err());
        Ok(())
    }

    #[test]
    fn test_cannot_deposit_when_locked() {
        let mut client = ClientBalance::new(1);
        client.locked = true;
        let res = client.deposit("100");
        assert!(res.is_err());
    }

    #[test]
    fn test_cannot_withdraw_when_locked() {
        let mut client = ClientBalance::new(1);
        client.locked = true;
        let res = client.withdraw("50");
        assert!(res.is_err());
    }
}

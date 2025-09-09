use crate::{
    errors::LedgerError,
    models::{ClientBalance, Transaction, TransactionType},
    traits::{Accounting, CSVExport, CSVWrite},
};
use anyhow::Result;
use std::{
    cell::{Cell, RefCell},
    collections::{HashMap, hash_map::Entry},
};

pub struct Accountant {
    clients: HashMap<u16, ClientBalance>,
    transactions: HashMap<u32, Transaction>,
    transaction_in_historical: Vec<u32>,
    transactions_rejected: Vec<u32>,
}

impl Accountant {
    pub fn new() -> Self {
        Self {
            clients: HashMap::new(),
            transactions: HashMap::new(),
            transaction_in_historical: Vec::new(),
            transactions_rejected: Vec::new(),
        }
    }
}

impl Accounting for Accountant {
    fn apply_bookkeeping(&mut self, transaction: Transaction) -> Result<()> {
        let transaction_id = *transaction.tx();

        match self.transactions.entry(transaction_id) {
            Entry::Vacant(entry) => {
                entry.insert(transaction.clone());
                self.transaction_in_historical.push(transaction_id);
            }
            Entry::Occupied(_) => match transaction.type_() {
                TransactionType::Deposit | TransactionType::Withdrawal => {
                    Err(LedgerError::TxDuplicated(transaction_id))?
                }
                _ => (),
            },
        }
        self.transaction_in_historical.push(transaction_id);

        let client_id = *transaction.client();

        let clients = RefCell::new(&mut self.clients);
        let transactions_rejected = RefCell::new(&mut self.transactions_rejected);
        let transactions = Cell::new(&self.transactions);

        clients
            .borrow_mut()
            .entry(client_id)
            .and_modify(|client| {
                match Self::update_client_balance(transactions.clone(), client, &transaction) {
                    Ok(_) => (),
                    Err(_) => transactions_rejected.borrow_mut().push(transaction_id),
                }
            })
            .or_insert_with(|| {
                let mut client = ClientBalance::new(client_id);
                match Self::update_client_balance(transactions, &mut client, &transaction) {
                    Ok(_) => (),
                    Err(_) => transactions_rejected.borrow_mut().push(transaction_id),
                }
                client
            });

        Ok(())
    }
}

impl CSVExport for Accountant {
    fn export(&self, writer: RefCell<&mut impl CSVWrite>) -> Result<()> {
        for client in self.clients.values() {
            writer.borrow_mut().write_record(client)?;
        }
        Ok(())
    }
}

impl Accountant {
    fn update_client_balance(
        transactions: Cell<&HashMap<u32, Transaction>>,
        client: &mut ClientBalance,
        tx: &Transaction,
    ) -> Result<()> {
        match tx.type_() {
            TransactionType::Deposit => client.deposit(tx.amount()),
            TransactionType::Withdrawal => client.withdraw(tx.amount()),
            TransactionType::Dispute => client.dispute(
                transactions
                    .get()
                    .get(tx.tx())
                    .ok_or(LedgerError::TxNotFound(*tx.tx()))?
                    .amount(),
            ),
            TransactionType::Resolve => client.resolve(
                transactions
                    .get()
                    .get(tx.tx())
                    .ok_or(LedgerError::TxNotFound(*tx.tx()))?
                    .amount(),
            ),
            TransactionType::Chargeback => client.chargeback(
                transactions
                    .get()
                    .get(tx.tx())
                    .ok_or(LedgerError::TxNotFound(*tx.tx()))?
                    .amount(),
            ),
        }?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::{Error as E, Result, anyhow};
    use rust_decimal::prelude::*;
    use std::cell::RefCell;

    struct MockCSVWriter {
        records: Vec<String>,
    }

    impl MockCSVWriter {
        fn new() -> Self {
            Self {
                records: Vec::new(),
            }
        }
    }

    impl CSVWrite for MockCSVWriter {
        fn write_record<T: serde::Serialize>(&mut self, record: &T) -> Result<()> {
            let line = serde_json::to_string(record)?;
            self.records.push(line);
            Ok(())
        }
    }

    #[test]
    fn test_accountant_export_writes_all_clients() -> Result<()> {
        let mut accountant = Accountant::new();
        accountant.clients.insert(1, ClientBalance::new(1));
        accountant.clients.insert(2, ClientBalance::new(2));

        let mut mock_writer = MockCSVWriter::new();
        let writer_ref = RefCell::new(&mut mock_writer);

        accountant.export(writer_ref)?;

        assert_eq!(mock_writer.records.len(), 2);

        let first = &mock_writer.records.first();
        assert!(first.is_some());

        let second = &mock_writer.records.get(1);
        assert!(second.is_some());

        Ok(())
    }

    fn create_transaction(tx: u32, client: u16, amount: &str, type_: &str) -> Result<Transaction> {
        let file_str = format!(
            "type,client,tx,amount\n{type_},{client},{tx},{amount}\n"
        );

        let mut reader = csv::ReaderBuilder::new()
            .has_headers(true)
            .from_reader(file_str.as_bytes());

        let transaction: Transaction = reader
            .deserialize::<Transaction>()
            .next()
            .ok_or(anyhow!("cannot serialize"))?
            .map_err(E::msg)?;

        Ok(transaction)
    }

    #[test]
    fn test_apply_deposit() -> Result<()> {
        let mut accountant = Accountant::new();
        let tx = create_transaction(1, 1, "100.0", "deposit")?;

        accountant.apply_bookkeeping(tx)?;

        let client = accountant.clients.get(&1).unwrap();
        assert_eq!(*client.available(), dec!(100.0));
        assert_eq!(*client.total(), dec!(100.0));
        Ok(())
    }

    #[test]
    fn test_apply_withdrawal() -> Result<()> {
        let mut accountant = Accountant::new();
        accountant.apply_bookkeeping(create_transaction(1, 1, "200.0", "deposit")?)?;
        accountant.apply_bookkeeping(create_transaction(2, 1, "50.0", "withdrawal")?)?;

        let client = accountant.clients.get(&1).unwrap();
        assert_eq!(*client.available(), dec!(150.0));
        assert_eq!(*client.total(), dec!(150.0));
        Ok(())
    }

    #[test]
    fn test_apply_dispute_and_resolve() -> Result<()> {
        let mut accountant = Accountant::new();
        accountant.apply_bookkeeping(create_transaction(1, 1, "300.0", "deposit")?)?;
        accountant.apply_bookkeeping(create_transaction(2, 1, "100.0", "deposit")?)?;
        accountant.apply_bookkeeping(create_transaction(1, 1, "", "dispute")?)?;

        let client = accountant.clients.get(&1).unwrap();
        assert_eq!(*client.available(), dec!(100.0));
        assert_eq!(*client.held(), dec!(300.0));
        assert_eq!(*client.total(), dec!(400.0));

        accountant.apply_bookkeeping(create_transaction(1, 1, "", "resolve")?)?;
        let client = accountant.clients.get(&1).unwrap();
        assert_eq!(*client.available(), dec!(400.0));
        assert_eq!(*client.total(), dec!(400.0));
        assert_eq!(*client.held(), dec!(0.0));
        Ok(())
    }

    #[test]
    fn test_apply_chargeback_locks_account() -> Result<()> {
        let mut accountant = Accountant::new();
        accountant.apply_bookkeeping(create_transaction(1, 1, "400.0", "deposit")?)?;
        accountant.apply_bookkeeping(create_transaction(1, 1, "", "dispute")?)?;
        accountant.apply_bookkeeping(create_transaction(2, 1, "200.0", "deposit")?)?;
        accountant.apply_bookkeeping(create_transaction(2, 1, "", "dispute")?)?;
        accountant.apply_bookkeeping(create_transaction(2, 1, "", "chargeback")?)?;

        let client = accountant.clients.get(&1).unwrap();
        assert_eq!(*client.total(), dec!(400.0));
        assert_eq!(*client.held(), dec!(400.0));
        assert_eq!(*client.available(), dec!(0.0));
        assert!(*client.locked());
        Ok(())
    }

    #[test]
    fn test_cannot_deposit_after_chargeback() -> Result<()> {
        let mut accountant = Accountant::new();
        accountant.apply_bookkeeping(create_transaction(1, 1, "400.0", "deposit")?)?;
        accountant.apply_bookkeeping(create_transaction(1, 1, "", "dispute")?)?;
        accountant.apply_bookkeeping(create_transaction(2, 1, "200.0", "deposit")?)?;
        accountant.apply_bookkeeping(create_transaction(2, 1, "", "dispute")?)?;
        accountant.apply_bookkeeping(create_transaction(2, 1, "", "chargeback")?)?;

        let client = accountant.clients.get(&1).unwrap();
        assert_eq!(*client.total(), dec!(400.0));
        assert_eq!(*client.held(), dec!(400.0));
        assert_eq!(*client.available(), dec!(0.0));
        assert!(*client.locked());

        accountant.apply_bookkeeping(create_transaction(3, 1, "500.0", "deposit")?)?;
        assert_eq!(accountant.transactions_rejected.len(), 1);
        assert_eq!(accountant.transactions_rejected[0], 3);
        Ok(())
    }
}

use anyhow::{anyhow, Result};
use rust_decimal::Decimal;
use std::collections::HashMap;

use crate::Transaction;
use serde::{Serialize, Serializer};

#[derive(Default, Serialize, Debug)]
pub struct Account {
    client: u16,
    #[serde(serialize_with = "serialize_with_fixed_digits")]
    available: Decimal,
    #[serde(serialize_with = "serialize_with_fixed_digits")]
    held: Decimal,
    #[serde(serialize_with = "serialize_with_fixed_digits")]
    total: Decimal,
    locked: bool,
    #[serde(skip)]
    deposits: HashMap<u32, Transaction>,
}

// This is here so that we can keep the output to 4 decimal places.
fn serialize_with_fixed_digits<S>(num: &Decimal, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(&format!("{:.4}", num))
}

impl Account {
    pub fn new(client: u16) -> Self {
        Account {
            client,
            ..Default::default()
        }
    }
    // A deposit should increase available funds.
    // If the account has been "frozen" (i.e locked),
    // no deposits are allowed.
    fn deposit(&mut self, amount: Decimal) -> Result<()> {
        if self.locked {
            return Err(anyhow!("account {} locked", self.client));
        }
        self.available += amount;
        self.total = self.available + self.held;
        Ok(())
    }
    // A withdrawal should decrease available funds.
    // If there is insufficient funds or the account has been
    // "frozen" (i.e locked), no withdrawals are allowed.
    fn withdraw(&mut self, amount: Decimal) -> Result<()> {
        if self.locked {
            return Err(anyhow!("account {} locked", self.client));
        }
        if self.available < amount {
            return Err(anyhow!(
                "account {}: insufficient funds, want {:.4}, have {:.4}",
                self.client,
                amount,
                self.available
            ));
        }
        self.available -= amount;
        self.total = self.available + self.held;
        Ok(())
    }
    // A dispute results in the disputed amount being held
    // which means the available funds should decrease by
    // the disputed amount and the held amount increase by
    // the same.
    fn dispute(&mut self, amount: Decimal) -> Result<()> {
        self.held += amount;
        self.available -= amount;
        self.total = self.available + self.held;
        Ok(())
    }
    // Resolving a dispute results in reversing the dispute, i.e
    // the account should "revert" the dispute. We do so here by
    // negating the input to dispute.
    fn resolve(&mut self, amount: Decimal) -> Result<()> {
        self.dispute(-amount)
    }
    // A chargeback should result in the account being immediately
    // frozen (i.e locked), the dispute should be reversed and, importantly,
    // a withdrawal of the disputed amount should happen.
    fn chargeback(&mut self, amount: Decimal) -> Result<()> {
        self.resolve(amount)?;
        self.withdraw(amount)?;
        self.lock()
    }
    fn lock(&mut self) -> Result<()> {
        self.locked = true;
        Ok(())
    }

    pub fn apply_transaction(&mut self, transaction: Transaction) -> Result<()> {
        match transaction {
            // Only deposits can be disputed, resolved or chargeback:ed so it is the only
            // type of transaction being tracked in the deposits field (a HashMap).
            Transaction::Deposit { tx, amount, .. } => {
                self.deposits.insert(tx, transaction);
                self.deposit(amount.ok_or_else(|| anyhow!("transaction {} missing amount", tx))?)
            }
            Transaction::Withdrawal { tx, amount, .. } => {
                self.withdraw(amount.ok_or_else(|| anyhow!("transaction {} missing amount", tx))?)
            }
            // Disputes don't have their own unique tx id but rather contain the tx id
            // they refer to. We fetch a transaction from the deposits hashmap via that id
            // and dispute it. See the private dispute method.
            // We also use the dispute method on the transaction itself which will turn
            // the deposit into a dispute.
            Transaction::Dispute { tx, .. } => {
                let transaction = self.deposits.get_mut(&tx).ok_or_else(|| {
                    anyhow!("dispute refers to non-existent deposit transaction {}", tx)
                })?;
                let amount = transaction
                    .get_amount()
                    .ok_or_else(|| anyhow!("transaction {} missing amount", tx))?;
                transaction.dispute(self.client)?;
                self.dispute(amount)
            }
            // Resolves don't have their own unique tx id but rather contain the tx id
            // they refer to. We fetch a transaction from the deposits hashmap via that id
            // and resolve it. Please note that that deposit should previously have turned
            // into a dispute. If not, this will fail.
            Transaction::Resolve { tx, .. } => {
                let transaction = self.deposits.get_mut(&tx).ok_or_else(|| {
                    anyhow!("resolve refers to non-existent dispute transaction {}", tx)
                })?;
                let amount = transaction
                    .get_amount()
                    .ok_or_else(|| anyhow!("transaction missing amount"))?;
                transaction.resolve(self.client)?;
                self.resolve(amount)
            }
            // Chargebacks don't have their own unique tx id but rather contain the tx id
            // they refer to. We fetch a transaction from the deposits hashmap via that id
            // and chargeback it. Please note that that deposit should previously have turned
            // into a dispute. If not (i.e it is not a dispute), this will fail.
            Transaction::Chargeback { tx, .. } => {
                let transaction = self.deposits.get_mut(&tx).ok_or_else(|| {
                    anyhow!(
                        "chargeback refers to non-existent dispute transaction {}",
                        tx
                    )
                })?;
                let amount = transaction
                    .get_amount()
                    .ok_or_else(|| anyhow!("transaction missing amount"))?;
                transaction.chargeback(self.client)?;
                self.chargeback(amount)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Account;
    use crate::Transaction;
    use anyhow::Result;

    #[test]
    fn a_new_account_is_empty() -> Result<()> {
        let account = Account::new(1);
        assert_eq!(account.available, 0.into());
        assert_eq!(account.held, 0.into());
        assert_eq!(account.total, 0.into());
        Ok(())
    }

    #[test]
    fn a_deposit_transaction_deposits_money_in_the_account_it_is_applied_to() -> Result<()> {
        let mut account = Account::new(1);
        account.apply_transaction(Transaction::Deposit {
            amount: Some(50.into()),
            client: 1,
            tx: 1,
        })?;
        assert_eq!(account.available, account.total);
        assert_eq!(account.held, 0.into());
        assert_eq!(account.available, 50.into());
        Ok(())
    }

    #[test]
    fn a_withdrawal_transaction_withdraws_money_from_the_account_it_is_applied_to() -> Result<()> {
        let mut account = Account::new(1);
        account.apply_transaction(Transaction::Deposit {
            amount: Some(100.into()),
            client: 1,
            tx: 1,
        })?;
        account.apply_transaction(Transaction::Withdrawal {
            amount: Some(50.into()),
            client: 1,
            tx: 2,
        })?;
        assert_eq!(account.available, account.total);
        assert_eq!(account.held, 0.into());
        assert_eq!(account.available, 50.into());
        Ok(())
    }

    #[test]
    fn a_withdrawal_transaction_fails_silently_when_there_is_insufficient_funds_in_the_account_it_is_applied_to(
    ) -> Result<()> {
        let mut account = Account::new(1);
        account.apply_transaction(Transaction::Deposit {
            amount: Some(100.into()),
            client: 1,
            tx: 1,
        })?;
        assert!(account
            .apply_transaction(Transaction::Withdrawal {
                amount: Some(101.into()),
                client: 1,
                tx: 2,
            })
            .is_err(),);
        assert_eq!(account.available, account.total);
        assert_eq!(account.held, 0.into());
        assert_eq!(account.available, 100.into());
        Ok(())
    }

    #[test]
    fn a_withdrawal_transaction_fails_silently_when_the_account_it_is_applied_to_is_locked(
    ) -> Result<()> {
        let mut account = Account::new(1);
        account.apply_transaction(Transaction::Deposit {
            amount: Some(100.into()),
            client: 1,
            tx: 1,
        })?;
        account.lock()?;
        assert!(account
            .apply_transaction(Transaction::Withdrawal {
                amount: Some(50.into()),
                client: 1,
                tx: 2,
            })
            .is_err());
        assert_eq!(account.available, account.total);
        assert_eq!(account.held, 0.into());
        assert_eq!(account.available, 100.into());
        Ok(())
    }

    #[test]
    fn a_dispute_transaction_holds_the_given_amount_in_the_account_it_is_applied_to() -> Result<()>
    {
        let mut account = Account::new(1);
        account.apply_transaction(Transaction::Deposit {
            amount: Some(70.into()),
            client: 1,
            tx: 1,
        })?;
        account.apply_transaction(Transaction::Deposit {
            amount: Some(30.into()),
            client: 1,
            tx: 2,
        })?;
        assert_eq!(account.available, account.total);
        assert_eq!(account.held, 0.into());
        assert_eq!(account.available, 100.into());
        account.apply_transaction(Transaction::Dispute {
            amount: None,
            client: 1,
            tx: 2,
        })?;
        assert_eq!(account.held, 30.into());
        assert_eq!(account.available, 70.into());
        assert_eq!(account.total, account.held + account.available);
        Ok(())
    }

    #[test]
    fn a_resolve_transaction_unholds_the_given_amount_in_the_account_it_is_applied_to() -> Result<()>
    {
        let mut account = Account::new(1);
        account.apply_transaction(Transaction::Deposit {
            amount: Some(100.into()),
            client: 1,
            tx: 1,
        })?;
        account.apply_transaction(Transaction::Deposit {
            amount: Some(30.into()),
            client: 1,
            tx: 2,
        })?;
        assert_eq!(account.held, 0.into());
        assert_eq!(account.available, 130.into());
        assert_eq!(account.total, account.held + account.available);
        account.apply_transaction(Transaction::Dispute {
            amount: None,
            client: 1,
            tx: 1,
        })?;
        assert_eq!(account.held, 100.into());
        assert_eq!(account.available, 30.into());
        assert_eq!(account.total, account.held + account.available);
        account.apply_transaction(Transaction::Resolve {
            amount: None,
            client: 1,
            tx: 1,
        })?;
        assert_eq!(account.held, 0.into());
        assert_eq!(account.available, 130.into());
        assert_eq!(account.total, account.held + account.available);
        Ok(())
    }

    #[test]
    fn a_chargeback_transaction_withdraws_amount_and_freezes_the_account_it_is_applied_to(
    ) -> Result<()> {
        let mut account = Account::new(1);
        account.apply_transaction(Transaction::Deposit {
            amount: Some(100.into()),
            client: 1,
            tx: 1,
        })?;
        account.apply_transaction(Transaction::Deposit {
            amount: Some(20.into()),
            client: 1,
            tx: 2,
        })?;
        assert_eq!(account.available, account.total);
        assert_eq!(account.held, 0.into());
        assert_eq!(account.available, 120.into());
        assert!(!account.locked);
        account.apply_transaction(Transaction::Dispute {
            amount: None,
            client: 1,
            tx: 1,
        })?;
        assert_eq!(account.held, 100.into());
        assert_eq!(account.available, 20.into());
        assert_eq!(account.total, account.available + account.held);
        assert!(!account.locked);
        account.apply_transaction(Transaction::Chargeback {
            amount: None,
            client: 1,
            tx: 1,
        })?;
        assert_eq!(account.available, account.total);
        assert_eq!(account.held, 0.into());
        assert_eq!(account.available, 20.into());
        assert!(account.locked);
        Ok(())
    }
}

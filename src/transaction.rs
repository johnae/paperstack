use core::fmt;

use anyhow::{anyhow, Result};
use rust_decimal::Decimal;
use serde::Deserialize;

// Why do we have this "intermediate" representation?
// I.e why not deserialize directly into a Transaction?
// Because: https://github.com/BurntSushi/rust-csv/issues/211
#[derive(Deserialize, Debug)]
struct TransactionEntry {
    #[serde(rename = "type")]
    kind: TransactionEntryKind,
    client: u16,
    tx: u32,
    amount: Option<Decimal>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "snake_case")]
enum TransactionEntryKind {
    Deposit,
    Withdrawal,
    Dispute,
    Resolve,
    Chargeback,
}

#[derive(Deserialize, Debug, PartialEq)]
#[serde(from = "TransactionEntry")]
pub enum Transaction {
    Deposit {
        client: u16,
        tx: u32,
        amount: Option<Decimal>,
    },
    Withdrawal {
        client: u16,
        tx: u32,
        amount: Option<Decimal>,
    },
    Dispute {
        client: u16,
        tx: u32,
        amount: Option<Decimal>,
    },
    Resolve {
        client: u16,
        tx: u32,
        amount: Option<Decimal>,
    },
    Chargeback {
        client: u16,
        tx: u32,
        amount: Option<Decimal>,
    },
}

impl Eq for Transaction {}

impl fmt::Display for Transaction {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Transaction::Deposit { client, tx, amount } => write!(
                f,
                "Deposit [ client: {}, tx: {}, amount: {:?} ]",
                client, tx, amount
            ),
            Transaction::Withdrawal { client, tx, amount } => write!(
                f,
                "Withdrawal [ client: {}, tx: {}, amount: {:?} ]",
                client, tx, amount
            ),
            Transaction::Dispute { client, tx, amount } => write!(
                f,
                "Dispute [ client: {}, tx: {}, amount: {:?} ]",
                client, tx, amount
            ),
            Transaction::Resolve { client, tx, amount } => write!(
                f,
                "Resolve [ client: {}, tx: {}, amount: {:?} ]",
                client, tx, amount
            ),
            Transaction::Chargeback { client, tx, amount } => write!(
                f,
                "Chargeback [ client: {}, tx: {}, amount: {:?} ]",
                client, tx, amount
            ),
        }
    }
}

impl From<TransactionEntry> for Transaction {
    fn from(te: TransactionEntry) -> Self {
        match te.kind {
            TransactionEntryKind::Deposit => Transaction::Deposit {
                client: te.client,
                tx: te.tx,
                amount: te.amount,
            },
            TransactionEntryKind::Withdrawal => Transaction::Withdrawal {
                client: te.client,
                tx: te.tx,
                amount: te.amount,
            },
            TransactionEntryKind::Dispute => Transaction::Dispute {
                client: te.client,
                tx: te.tx,
                amount: te.amount,
            },
            TransactionEntryKind::Resolve => Transaction::Resolve {
                client: te.client,
                tx: te.tx,
                amount: te.amount,
            },
            TransactionEntryKind::Chargeback => Transaction::Chargeback {
                client: te.client,
                tx: te.tx,
                amount: te.amount,
            },
        }
    }
}

impl Transaction {
    pub fn get_client(&self) -> &u16 {
        match self {
            Transaction::Deposit { client, .. } => client,
            Transaction::Withdrawal { client, .. } => client,
            Transaction::Dispute { client, .. } => client,
            Transaction::Resolve { client, .. } => client,
            Transaction::Chargeback { client, .. } => client,
        }
    }
    pub fn get_amount(&self) -> &Option<Decimal> {
        match self {
            Transaction::Deposit { amount, .. } => amount,
            Transaction::Withdrawal { amount, .. } => amount,
            Transaction::Dispute { amount, .. } => amount,
            Transaction::Resolve { amount, .. } => amount,
            Transaction::Chargeback { amount, .. } => amount,
        }
    }
    // Only deposits can be disputed.
    pub fn dispute(&mut self, from_client: u16) -> Result<()> {
        if let Transaction::Deposit { client, tx, amount } = self {
            if *client != from_client {
                return Err(anyhow!(
                    "cannot dispute transaction {} belonging to client {} as client {}",
                    tx,
                    client,
                    from_client
                ));
            };
            *self = Transaction::Dispute {
                client: *client,
                tx: *tx,
                amount: *amount,
            };
            return Ok(());
        }
        Err(anyhow!(
            "only deposits can be disputed but {} is not a deposit",
            self
        ))
    }
    // Only disputed transactions can be resolved.
    pub fn resolve(&mut self, from_client: u16) -> Result<()> {
        if let Transaction::Dispute { client, tx, amount } = self {
            if *client != from_client {
                return Err(anyhow!(
                    "cannot resolve transaction {} belonging to client {} as client {}",
                    tx,
                    client,
                    from_client
                ));
            };
            *self = Transaction::Resolve {
                client: *client,
                tx: *tx,
                amount: *amount,
            };
            return Ok(());
        }
        Err(anyhow!(
            "only disputes can be resolved but {} is not a dispute",
            self
        ))
    }
    // Only disputed transactions can be chargeback:ed.
    pub fn chargeback(&mut self, from_client: u16) -> Result<()> {
        if let Transaction::Dispute { client, tx, amount } = self {
            if *client != from_client {
                return Err(anyhow!(
                    "cannot chargeback transaction {} belonging to client {} as client {}",
                    tx,
                    client,
                    from_client
                ));
            };
            *self = Transaction::Chargeback {
                client: *client,
                tx: *tx,
                amount: *amount,
            };
            return Ok(());
        }
        Err(anyhow!(
            "only disputes can be chargeback:ed but {} is not a dispute",
            self
        ))
    }
}

#[cfg(test)]
mod tests {

    use super::Transaction;

    #[test]
    fn a_deposit_can_be_turned_into_a_dispute() {
        let mut transaction = Transaction::Deposit {
            client: 1,
            tx: 1,
            amount: Some(10.into()),
        };
        assert!(transaction.dispute(1).is_ok());
        assert_eq!(
            transaction,
            Transaction::Dispute {
                client: 1,
                tx: 1,
                amount: Some(10.into())
            }
        );
    }

    #[test]
    fn disputing_a_deposit_using_the_wrong_client_id_fails() {
        let mut transaction = Transaction::Deposit {
            amount: Some(10.into()),
            client: 1,
            tx: 1,
        };
        assert!(transaction.dispute(2).is_err());
        assert_eq!(
            transaction,
            Transaction::Deposit {
                amount: Some(10.into()),
                client: 1,
                tx: 1,
            }
        );
        assert_eq!(transaction.get_amount(), &Some(10.into()));
    }

    #[test]
    fn a_deposit_cannot_be_turned_into_transactions_other_than_disputes() {
        let mut transaction = Transaction::Deposit {
            amount: Some(10.into()),
            client: 1,
            tx: 1,
        };
        assert_eq!(
            transaction,
            Transaction::Deposit {
                amount: Some(10.into()),
                client: 1,
                tx: 1,
            }
        );
        assert!(transaction.chargeback(1).is_err());
        assert_eq!(
            transaction,
            Transaction::Deposit {
                amount: Some(10.into()),
                client: 1,
                tx: 1,
            }
        );
    }

    #[test]
    fn a_dispute_can_be_turned_into_a_resolve() {
        let mut transaction = Transaction::Dispute {
            amount: Some(10.into()),
            client: 1,
            tx: 1,
        };
        assert!(transaction.resolve(1).is_ok());
        assert_eq!(
            transaction,
            Transaction::Resolve {
                amount: Some(10.into()),
                client: 1,
                tx: 1,
            }
        );
    }

    #[test]
    fn resolving_a_dispute_using_the_wrong_client_id_fails() {
        let mut transaction = Transaction::Dispute {
            amount: Some(10.into()),
            client: 1,
            tx: 1,
        };
        assert!(transaction.resolve(2).is_err());
        assert_eq!(
            transaction,
            Transaction::Dispute {
                amount: Some(10.into()),
                client: 1,
                tx: 1,
            }
        );
    }

    #[test]
    fn a_dispute_can_be_turned_into_a_chargeback() {
        let mut transaction = Transaction::Dispute {
            amount: Some(10.into()),
            client: 1,
            tx: 1,
        };
        assert!(transaction.chargeback(1).is_ok());
        assert_eq!(
            transaction,
            Transaction::Chargeback {
                amount: Some(10.into()),
                client: 1,
                tx: 1,
            }
        );
    }

    #[test]
    fn chargebacking_a_dispute_using_the_wrong_client_id_fails() {
        let mut transaction = Transaction::Dispute {
            amount: Some(10.into()),
            client: 1,
            tx: 1,
        };
        assert!(transaction.chargeback(2).is_err());
        assert_eq!(
            transaction,
            Transaction::Dispute {
                amount: Some(10.into()),
                client: 1,
                tx: 1,
            }
        );
    }

    #[test]
    fn a_chargeback_cannot_be_turned_into_other_kinds_of_transactions() {
        let mut transaction = Transaction::Chargeback {
            amount: Some(10.into()),
            client: 1,
            tx: 1,
        };
        assert!(transaction.resolve(1).is_err());
        assert_eq!(
            transaction,
            Transaction::Chargeback {
                amount: Some(10.into()),
                client: 1,
                tx: 1,
            }
        );

        assert!(transaction.dispute(1).is_err());
        assert_eq!(
            transaction,
            Transaction::Chargeback {
                client: 1,
                tx: 1,
                amount: Some(10.into()),
            }
        );

        assert!(transaction.chargeback(1).is_err());
        assert_eq!(
            transaction,
            Transaction::Chargeback {
                client: 1,
                tx: 1,
                amount: Some(10.into()),
            }
        );
    }
}

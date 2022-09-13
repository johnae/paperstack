mod account;
use account::Account;

mod transaction;
use transaction::Transaction;

use csv::Trim;
use std::{collections::HashMap, env, error::Error, ffi::OsString, io};

fn main() -> anyhow::Result<(), Box<dyn Error>> {
    // Assume the only argument is the path to a csv containing transactions, fail if no path is provided
    let csv_path = match env::args_os().nth(1) {
        None => Err::<OsString, Box<dyn Error>>(From::from("expected 1 argument, but got none")),
        Some(file_path) => Ok(file_path),
    }?;

    // Create a ReaderBuilder so that we may configure it to allow whitespace.
    let mut reader = csv::ReaderBuilder::new()
        .trim(Trim::All)
        .from_path(csv_path)?;

    // Read every transaction in the order they come in - this is the only ordering available to us as tx ids,
    // while unique u32:s, don't actually imply any ordering.
    let mut accounts = HashMap::<u16, Account>::new();
    for result in reader.deserialize::<Transaction>() {
        let tx = result.expect("transaction to be deserialized");
        // Here we're trying to either find an account with the correct client id or create a new one
        // if one doesn't exist.
        let account = accounts
            .entry(*tx.get_client())
            .or_insert_with(|| Account::new(*tx.get_client()));

        // Then we apply the transaction that was deserialized to the account
        // in question.
        // If the transaction fails we print the error to stderr.
        if let Err(e) = account.apply_transaction(tx) {
            eprintln!("{}", e);
        }
    }
    // Finally we write our updated accounts to stdout.
    let mut csv_writer = csv::Writer::from_writer(io::stdout());
    for (_, account) in accounts {
        csv_writer
            .serialize(account)
            .expect("account to be serialized");
    }
    csv_writer.flush()?;
    Ok(())
}

use crate::{
    client::{ClientAccount, TransactionExecutionError},
    transaction::{Transaction, TransactionExecutor},
};
use im::HashMap;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    ExecutionError(#[from] TransactionExecutionError),
}

#[derive(Default, Debug, Clone)]
pub struct Ledger {
    clients: HashMap<u16, ClientAccount>,
}

impl Ledger {
    pub fn iter(&self) -> impl Iterator<Item = &ClientAccount> {
        self.clients.values()
    }
}

impl TransactionExecutor<Transaction> for Ledger {
    type TransactionError = Error;

    fn execute(mut self, transaction: Transaction) -> Result<Self, Self::TransactionError> {
        let client_id = transaction.get_client_id();

        let client = transaction.execute(
            self.clients
                .get(&client_id)
                .cloned()
                .unwrap_or_else(|| ClientAccount::new(client_id)),
        )?;

        self.clients = self.clients.update(client_id, client);

        Ok(self)
    }
}

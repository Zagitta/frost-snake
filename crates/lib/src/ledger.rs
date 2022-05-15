use crate::{
    client::{ClientAccount, TransactionExecutionError},
    transaction::{Transaction, TransactionExecutor},
    Deposit, DepositState, UCurrency,
};
use std::collections::HashMap;
use thiserror::Error;

#[derive(Default, Debug, Clone, PartialEq)]
pub struct Ledger {
    clients: HashMap<u16, (ClientAccount, HashMap<u32, (UCurrency, DepositState)>)>,
}

impl Ledger {
    pub fn iter(&self) -> impl Iterator<Item = &ClientAccount> {
        self.clients.values().map(|(client, _)| client)
    }
}

impl TransactionExecutor<Transaction> for &mut Ledger {
    type TransactionError = TransactionExecutionError;

    fn execute(self, transaction: Transaction) -> Result<Self, Self::TransactionError> {
        let client_id = transaction.get_client_id();
        let (client, deposits) = self
            .clients
            .entry(client_id)
            .or_insert_with(|| (ClientAccount::new(client_id), HashMap::default()));
        match transaction {
            Transaction::Deposit(d) => {
                let tx = d.tx;
                let amount = d.amount;
                *client = TransactionExecutor::execute(client.clone(), d)?;
                deposits.insert(tx, (amount, DepositState::Ok));
            }
            Transaction::Dispute(d) => {
                let (amount, state) = deposits
                    .get_mut(&d.tx)
                    .ok_or(TransactionExecutionError::DepositNotFound(d.tx))?;
                let (new_client, _, new_state) =
                    TransactionExecutor::execute((client.clone(), *amount, *state), d)?;
                *state = new_state;
                *client = new_client;
            }
            Transaction::ChargeBack(c) => {
                let (amount, state) = deposits
                    .get_mut(&c.tx)
                    .ok_or(TransactionExecutionError::DepositNotFound(c.tx))?;
                let (new_client, _, new_state) =
                    TransactionExecutor::execute((client.clone(), *amount, *state), c)?;
                *state = new_state;
                *client = new_client;
            }
            Transaction::Resolve(r) => {
                let (amount, state) = deposits
                    .get_mut(&r.tx)
                    .ok_or(TransactionExecutionError::DepositNotFound(r.tx))?;
                let (new_client, _, new_state) =
                    TransactionExecutor::execute((client.clone(), *amount, *state), r)?;
                *state = new_state;
                *client = new_client;
            }
            Transaction::Withdrawal(w) => {
                *client = TransactionExecutor::execute(client.clone(), w)?
            }
        }

        Ok(self)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::ClientAccount;
    use crate::transaction::Transaction;
    use crate::{
        Deposit, DepositState, Dispute, Ledger, TransactionExecutionError, TransactionExecutor,
        UCurrency, Withdrawal,
    };
    use fixed_macro::types::I48F16 as icur;
    use fixed_macro::types::U48F16 as ucur;

    const client: u16 = 1;
    const tx: u32 = 1;
    const amount: UCurrency = ucur!(1);

    #[test]
    fn can_deposit_then_dispute() {
        let mut ledger = Ledger::default();
        let deposit = Transaction::new_deposit(tx, client, amount);
        let dispute = Transaction::new_dispute(tx, client);
        assert_eq!(
            ledger
                .execute(deposit.clone())
                .unwrap()
                .execute(dispute)
                .unwrap(),
            &mut Ledger {
                clients: HashMap::from([(
                    client,
                    (
                        ClientAccount {
                            id: client,
                            held: amount,
                            available: icur!(0),
                            locked: false,
                        },
                        HashMap::from([(tx, (amount, DepositState::Disputed))])
                    )
                )])
            }
        )
    }

    #[test]
    fn can_deposit_then_dispute_and_resolve() {
        let mut ledger = Ledger::default();

        let deposit = Transaction::new_deposit(tx, client, amount);
        let dispute = Transaction::new_dispute(tx, client);
        let resolve = Transaction::new_resolve(tx, client);

        //deposit -> dispute -> resolve should == deposit
        assert_eq!(
            ledger
                .execute(deposit)
                .unwrap()
                .execute(dispute)
                .unwrap()
                .execute(resolve),
            Ok(&mut Ledger {
                clients: HashMap::from([(
                    client,
                    (
                        ClientAccount {
                            id: client,
                            held: ucur!(0),
                            available: icur!(1),
                            locked: false,
                        },
                        HashMap::from([(tx, (amount, DepositState::Ok))])
                    )
                )])
            })
        );
    }

    #[test]
    fn cant_charge_back_invalid_transaction_id() {
        let mut ledger = Ledger::default();

        assert_eq!(
            ledger.execute(Transaction::new_charge_back(tx, client)),
            Err(TransactionExecutionError::DepositNotFound(tx))
        );
    }
    #[test]
    fn cant_resolve_undisputed_transaction() {
        let mut ledger = Ledger::default();
        let deposit = Transaction::new_deposit(tx, client, amount);
        let resolve = Transaction::new_resolve(tx, client);

        assert_eq!(
            ledger.execute(deposit).unwrap().execute(resolve),
            Err(TransactionExecutionError::InvalidDepositState {
                tx,
                expected_state: DepositState::Disputed,
                actual_state: DepositState::Ok
            })
        )
    }

    #[test]
    fn deposit_withdraw_charge_back_gives_negative_balance() {
        let mut ledger = Ledger::default();
        let deposit = Transaction::new_deposit(tx, client, amount);
        let withdrawal = Transaction::new_withdrawal(2, client, amount);
        let dispute = Transaction::new_dispute(tx, client);
        let charge_back = Transaction::new_charge_back(tx, client);

        assert_eq!(
            ledger
                .execute(deposit)
                .unwrap()
                .execute(withdrawal)
                .unwrap()
                .execute(dispute)
                .unwrap()
                .execute(charge_back),
            Ok(&mut Ledger {
                clients: HashMap::from([(
                    client,
                    (
                        ClientAccount {
                            id: client,
                            held: ucur!(0),
                            available: icur!(-1),
                            locked: true,
                        },
                        HashMap::from([(tx, (amount, DepositState::ChargedBack))])
                    )
                )])
            })
        )
    }

    #[test]
    fn cant_dispute_withdrawal() {
        let mut ledger = Ledger::default();
        let deposit = Transaction::new_deposit(tx, client, amount);
        let withdrawal = Transaction::new_withdrawal(tx + 1, client, amount);
        let dispute = Transaction::new_dispute(tx + 1, client);

        assert_eq!(
            ledger
                .execute(deposit)
                .unwrap()
                .execute(withdrawal)
                .unwrap()
                .execute(dispute),
            Err(TransactionExecutionError::DepositNotFound(tx + 1))
        )
    }
    #[test]
    fn cant_charge_back_multiple_times() {
        let mut ledger = Ledger::default();
        let deposit = Transaction::new_deposit(tx, client, amount);
        let dispute = Transaction::new_dispute(tx, client);
        let charge_back = Transaction::new_charge_back(tx, client);

        assert_eq!(
            ledger
                .execute(deposit)
                .unwrap()
                .execute(dispute)
                .unwrap()
                .execute(charge_back.clone())
                .unwrap()
                .execute(charge_back),
            Err(TransactionExecutionError::InvalidDepositState {
                tx,
                expected_state: DepositState::Disputed,
                actual_state: DepositState::ChargedBack
            })
        )
    }
}

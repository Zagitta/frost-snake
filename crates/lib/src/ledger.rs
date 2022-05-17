use crate::{
    client::{ClientAccount, TransactionExecutionError},
    transaction::Transaction,
    UCurrency,
};
use std::collections::{
    hash_map::Entry::{Occupied, Vacant},
    HashMap,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum DepositState {
    Ok,
    Disputed,
    ChargedBack,
}

impl std::fmt::Display for DepositState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            DepositState::Ok => "Ok",
            DepositState::Disputed => "Disputed",
            DepositState::ChargedBack => "ChargedBack",
        })
    }
}
#[derive(Default, Debug, Clone, PartialEq)]

struct ClientAccountAndDeposits {
    account: ClientAccount,
    deposits: HashMap<u32, (UCurrency, DepositState)>,
}

impl ClientAccountAndDeposits {
    pub fn new(client: u16) -> Self {
        Self {
            account: ClientAccount::new(client),
            deposits: Default::default(),
        }
    }
}

#[derive(Default, Debug, Clone, PartialEq)]
pub struct Ledger {
    clients: HashMap<u16, ClientAccountAndDeposits>,
}

fn get_deposit_and_state_mut(
    deposits: &mut HashMap<u32, (UCurrency, DepositState)>,
    tx: u32,
) -> Result<(UCurrency, &mut DepositState), TransactionExecutionError> {
    deposits
        .get_mut(&tx)
        .ok_or(TransactionExecutionError::DepositNotFound(tx))
        .map(|(amount, state)| (*amount, state))
}

impl Ledger {
    pub fn iter(&self) -> impl Iterator<Item = &ClientAccount> {
        self.clients.values().map(|client| &client.account)
    }

    pub fn execute(
        &mut self,
        transaction: Transaction,
    ) -> Result<&mut Self, TransactionExecutionError> {
        let client_id = transaction.get_client_id();
        let ClientAccountAndDeposits { account, deposits } = self
            .clients
            .entry(client_id)
            .or_insert_with(|| ClientAccountAndDeposits::new(client_id));

        match transaction {
            Transaction::Deposit(d) => {
                let tx = d.tx;
                let amount = d.amount;

                let ent = deposits.entry(tx);

                match ent {
                    Occupied(_) => return Err(TransactionExecutionError::DuplicateDeposit(tx)),
                    Vacant(ent) => {
                        *account = account.deposit(d)?;
                        ent.insert((amount, DepositState::Ok));
                    }
                }
            }
            Transaction::Dispute(d) => {
                let (amount, state) = get_deposit_and_state_mut(deposits, d.tx)?;
                (*account, *state) = account.dispute(d, amount, *state)?;
            }
            Transaction::ChargeBack(c) => {
                let (amount, state) = get_deposit_and_state_mut(deposits, c.tx)?;

                (*account, *state) = account.charge_back(c, amount, *state)?;
            }
            Transaction::Resolve(r) => {
                let (amount, state) = get_deposit_and_state_mut(deposits, r.tx)?;

                (*account, *state) = account.resolve(r, amount, *state)?;
            }
            Transaction::Withdrawal(w) => {
                *account = account.withdraw(w)?;
            }
        }

        Ok(self)
    }
}

#[cfg(test)]
mod tests {
    use super::{ClientAccount, ClientAccountAndDeposits};
    use crate::transaction::Transaction;
    use crate::{icur, ucur, DepositState, Ledger, TransactionExecutionError, UCurrency};
    use std::collections::HashMap;

    //make it easier to construct stuff
    #[allow(non_upper_case_globals)]
    const client: u16 = 1;
    #[allow(non_upper_case_globals)]
    const tx: u32 = 1;
    #[allow(non_upper_case_globals)]
    const amount: UCurrency = ucur!(1);

    #[test]
    fn can_deposit() {
        let deposit = Transaction::new_deposit(tx, client, amount);
        assert_eq!(
            Ledger::default().execute(deposit),
            Ok(&mut Ledger {
                clients: HashMap::from([(
                    client,
                    ClientAccountAndDeposits {
                        account: ClientAccount {
                            id: client,
                            held: ucur!(0),
                            available: icur!(1),
                            locked: false,
                        },
                        deposits: HashMap::from([(tx, (amount, DepositState::Ok))])
                    },
                )]),
            })
        );
    }

    #[test]
    fn can_deposit_then_dispute() {
        let deposit = Transaction::new_deposit(tx, client, amount);
        let dispute = Transaction::new_dispute(tx, client);
        assert_eq!(
            Ledger::default()
                .execute(deposit.clone())
                .unwrap()
                .execute(dispute)
                .unwrap(),
            &mut Ledger {
                clients: HashMap::from([(
                    client,
                    (ClientAccountAndDeposits {
                        account: ClientAccount {
                            id: client,
                            held: amount,
                            available: icur!(0),
                            locked: false,
                        },
                        deposits: HashMap::from([(tx, (amount, DepositState::Disputed))])
                    })
                )])
            }
        )
    }

    #[test]
    fn can_deposit_then_dispute_and_resolve() {
        let deposit = Transaction::new_deposit(tx, client, amount);
        let dispute = Transaction::new_dispute(tx, client);
        let resolve = Transaction::new_resolve(tx, client);

        //deposit -> dispute -> resolve should == deposit alone
        assert_eq!(
            Ledger::default()
                .execute(deposit.clone())
                .unwrap()
                .execute(dispute)
                .unwrap()
                .execute(resolve),
            Ledger::default().execute(deposit)
        );
    }

    #[test]
    fn cant_charge_back_invalid_transaction_id() {
        assert_eq!(
            Ledger::default().execute(Transaction::new_charge_back(tx, client)),
            Err(TransactionExecutionError::DepositNotFound(tx))
        );
    }
    #[test]
    fn cant_resolve_undisputed_transaction() {
        let deposit = Transaction::new_deposit(tx, client, amount);
        let resolve = Transaction::new_resolve(tx, client);

        assert_eq!(
            Ledger::default().execute(deposit).unwrap().execute(resolve),
            Err(TransactionExecutionError::InvalidDepositState {
                tx,
                expected_state: DepositState::Disputed,
                actual_state: DepositState::Ok
            })
        )
    }

    #[test]
    fn deposit_withdraw_charge_back_gives_negative_balance() {
        let deposit = Transaction::new_deposit(tx, client, amount);
        let withdrawal = Transaction::new_withdrawal(2, client, amount);
        let dispute = Transaction::new_dispute(tx, client);
        let charge_back = Transaction::new_charge_back(tx, client);

        assert_eq!(
            Ledger::default()
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
                    ClientAccountAndDeposits {
                        account: ClientAccount {
                            id: client,
                            held: ucur!(0),
                            available: icur!(-1),
                            locked: true,
                        },
                        deposits: HashMap::from([(tx, (amount, DepositState::ChargedBack))])
                    }
                )])
            })
        )
    }

    #[test]
    fn cant_dispute_withdrawal() {
        let deposit = Transaction::new_deposit(tx, client, amount);
        let withdrawal = Transaction::new_withdrawal(tx + 1, client, amount);
        let dispute = Transaction::new_dispute(tx + 1, client);

        assert_eq!(
            Ledger::default()
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
        let deposit = Transaction::new_deposit(tx, client, amount);
        let dispute = Transaction::new_dispute(tx, client);
        let charge_back = Transaction::new_charge_back(tx, client);

        assert_eq!(
            Ledger::default()
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

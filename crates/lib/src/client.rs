use crate::{
    transaction::{ChargeBack, Deposit, Dispute, Resolve, TransactionExecutor, Withdrawal},
    ICurrency, Transaction, UCurrency,
};
use az::CheckedAs;
use thiserror::Error;

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

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ClientAccount {
    pub id: u16,
    pub locked: bool,
    pub available: ICurrency,
    pub held: UCurrency,
    deposits: im::HashMap<u32, (Deposit, DepositState)>,
}

impl ClientAccount {
    pub fn new(id: u16) -> Self {
        Self {
            id,
            ..Default::default()
        }
    }

    pub fn total(&self) -> ICurrency {
        // Panic on the edge case that the client has +- 500 trillion assets...
        self.available.checked_add_unsigned(self.held).unwrap()
    }

    pub fn execute(self, transaction: Transaction) -> Result<Self, TransactionExecutionError> {
        transaction.execute(self)
    }
}

#[derive(Error, Debug, PartialEq, Eq)]
pub enum TransactionExecutionError {
    #[error("Inssuficient funds in account")]
    InsufficientFunds,
    #[error("The deposit tx = {0}, was not found")]
    DepositNotFound(u32),
    #[error(
        "The deposit tx = {tx} state is invalid, expected {expected_state} but was {actual_state}"
    )]
    InvalidDepositState {
        tx: u32,
        expected_state: DepositState,
        actual_state: DepositState,
    },
    #[error("Action resulted in an overflow")]
    Overflow,
    #[error("Action resulted in an underflow")]
    Underflow,
}

fn get_deposit(
    deposits: &mut im::HashMap<u32, (Deposit, DepositState)>,
    tx: u32,
) -> Result<(&Deposit, &mut DepositState), TransactionExecutionError> {
    let (deposit, state) = deposits
        .get_mut(&tx)
        .ok_or(TransactionExecutionError::DepositNotFound(tx))?;

    Ok((deposit, state))
}

fn get_deposit_with_state(
    deposits: &mut im::HashMap<u32, (Deposit, DepositState)>,
    tx: u32,
    expected_state: DepositState,
) -> Result<(&Deposit, &mut DepositState), TransactionExecutionError> {
    let (deposit, state) = get_deposit(deposits, tx)?;

    if *state != expected_state {
        return Err(TransactionExecutionError::InvalidDepositState {
            tx,
            expected_state,
            actual_state: *state,
        });
    }

    Ok((deposit, state))
}

impl TransactionExecutor<Deposit> for ClientAccount {
    type TransactionError = TransactionExecutionError;

    fn execute(mut self, deposit: Deposit) -> Result<Self, Self::TransactionError> {
        assert_eq!(self.id, deposit.client);

        let amount = deposit
            .amount
            .checked_as::<ICurrency>()
            .ok_or(TransactionExecutionError::Overflow)?;

        self.available = self
            .available
            .checked_add(amount)
            .ok_or(TransactionExecutionError::Overflow)?;

        self.deposits
            .insert(deposit.tx, (deposit, DepositState::Ok));
        Ok(self)
    }
}

impl TransactionExecutor<Withdrawal> for ClientAccount {
    type TransactionError = TransactionExecutionError;

    fn execute(mut self, withdrawal: Withdrawal) -> Result<Self, Self::TransactionError> {
        assert_eq!(self.id, withdrawal.client);

        let amount = withdrawal
            .amount
            .checked_as::<ICurrency>()
            .ok_or(TransactionExecutionError::Overflow)?;

        if self.available < amount {
            return Err(TransactionExecutionError::InsufficientFunds);
        }

        self.available = self
            .available
            .checked_sub(amount)
            .ok_or(TransactionExecutionError::Underflow)?;

        Ok(self)
    }
}

impl TransactionExecutor<Dispute> for ClientAccount {
    type TransactionError = TransactionExecutionError;

    fn execute(mut self, dispute: Dispute) -> Result<Self, Self::TransactionError> {
        assert_eq!(self.id, dispute.client);
        let tx = dispute.tx;

        let (deposit, state) = get_deposit_with_state(&mut self.deposits, tx, DepositState::Ok)?;

        let amount = deposit
            .amount
            .checked_as::<ICurrency>()
            .ok_or(TransactionExecutionError::Overflow)?;

        self.available = self
            .available
            .checked_sub(amount)
            .ok_or(TransactionExecutionError::Underflow)?;

        self.held = self
            .held
            .checked_add(deposit.amount)
            .ok_or(TransactionExecutionError::Overflow)?;

        *state = DepositState::Disputed;

        Ok(self)
    }
}

impl TransactionExecutor<Resolve> for ClientAccount {
    type TransactionError = TransactionExecutionError;

    fn execute(mut self, resolve: Resolve) -> Result<Self, Self::TransactionError> {
        assert_eq!(self.id, resolve.client);
        let tx = resolve.tx;

        let (deposit, state) =
            get_deposit_with_state(&mut self.deposits, tx, DepositState::Disputed)?;

        let amount = deposit
            .amount
            .checked_as::<ICurrency>()
            .ok_or(TransactionExecutionError::Overflow)?;

        self.available = self
            .available
            .checked_add(amount)
            .ok_or(TransactionExecutionError::Overflow)?;

        self.held = self
            .held
            .checked_sub(deposit.amount)
            .ok_or(TransactionExecutionError::Underflow)?;

        *state = DepositState::Ok;

        Ok(self)
    }
}

impl TransactionExecutor<ChargeBack> for ClientAccount {
    type TransactionError = TransactionExecutionError;

    fn execute(mut self, charge_back: ChargeBack) -> Result<Self, Self::TransactionError> {
        assert_eq!(self.id, charge_back.client);

        let (deposit, state) = get_deposit(&mut self.deposits, charge_back.tx)?;

        self.held = self
            .held
            .checked_sub(deposit.amount)
            .ok_or(TransactionExecutionError::Underflow)?;

        self.locked = true;
        *state = DepositState::ChargedBack;

        Ok(self)
    }
}

#[cfg(test)]
mod tests {
    use super::ClientAccount;
    use crate::transaction::Transaction;
    use crate::{Deposit, DepositState, TransactionExecutionError};
    use fixed_macro::types::I48F16 as icur;
    use fixed_macro::types::U48F16 as ucur;

    #[allow(non_upper_case_globals)] //make it easier to construct ClientAccount
    const client: u16 = 1;

    #[test]
    fn can_deposit() {
        let tx = 1;
        let amount = ucur!(1);
        let deposit = Transaction::new_deposit(tx, client, amount);
        assert_eq!(
            ClientAccount::new(client).execute(deposit),
            Ok(ClientAccount {
                id: client,
                held: ucur!(0),
                available: icur!(1),
                locked: false,
                deposits: im::hashmap! {
                    tx => (Deposit{amount, client, tx}, DepositState::Ok)
                }
            })
        );
    }

    #[test]
    fn can_withdraw() {
        assert_eq!(
            ClientAccount {
                available: icur!(1),
                id: client,
                ..Default::default()
            }
            .execute(Transaction::new_withdrawal(1, client, ucur!(1))),
            Ok(ClientAccount {
                id: client,
                locked: false,
                available: icur!(0),
                held: ucur!(0),
                deposits: Default::default()
            })
        );
    }

    #[test]
    fn can_deposit_then_dispute() {
        let ac = ClientAccount {
            id: client,
            ..Default::default()
        };
        let tx = 1;
        let amount = ucur!(1);
        let deposit = Transaction::new_deposit(tx, client, amount);
        let dispute = Transaction::new_dispute(tx, client);
        assert_eq!(
            ac.execute(deposit.clone()).unwrap().execute(dispute),
            Ok(ClientAccount {
                id: client,
                held: amount,
                available: icur!(0),
                locked: false,
                deposits: im::hashmap! {
                    tx => (Deposit{amount, client, tx}, DepositState::Disputed)
                }
            })
        )
    }

    #[test]
    fn can_deposit_then_dispute_and_resolve() {
        let ac = ClientAccount {
            id: client,
            ..Default::default()
        };
        let amount = ucur!(1);
        let deposit = Transaction::new_deposit(1, client, amount);
        let dispute = Transaction::new_dispute(deposit.get_tx(), client);
        let resolve = Transaction::new_resolve(deposit.get_tx(), client);

        let c1 = ac.execute(deposit).unwrap();
        let c2 = c1.clone().execute(dispute).unwrap();
        let c3 = c2.clone().execute(resolve).unwrap();

        //deposit -> dispute -> resolve should == deposit
        assert_eq!(c3, c1);
    }
    #[test]
    fn cant_charge_back_invalid_transaction_id() {
        let ac = ClientAccount {
            id: client,
            ..Default::default()
        };
        let tx = 1;

        assert_eq!(
            ac.clone().execute(Transaction::new_charge_back(tx, client)),
            Err(TransactionExecutionError::DepositNotFound(tx))
        );
    }
    #[test]
    fn cant_resolve_undisputed_transaction() {
        let ac = ClientAccount {
            id: client,
            ..Default::default()
        };
        let tx = 1;
        let amount = ucur!(1);
        let deposit = Transaction::new_deposit(tx, client, amount);
        let resolve = Transaction::new_resolve(deposit.get_tx(), client);

        assert_eq!(
            ac.execute(deposit).unwrap().execute(resolve),
            Err(TransactionExecutionError::InvalidDepositState {
                tx,
                expected_state: DepositState::Disputed,
                actual_state: DepositState::Ok
            })
        )
    }

    #[test]
    fn deposit_withdraw_charge_back_gives_negative_balance() {
        let ac = ClientAccount {
            id: client,
            ..Default::default()
        };
        let tx = 1;
        let amount = ucur!(1);
        let deposit = Transaction::new_deposit(tx, client, amount);
        let withdrawal = Transaction::new_withdrawal(2, client, amount);
        let dispute = Transaction::new_dispute(tx, client);
        let charge_back = Transaction::new_charge_back(tx, client);

        assert_eq!(
            ac.execute(deposit)
                .unwrap()
                .execute(withdrawal)
                .unwrap()
                .execute(dispute)
                .unwrap()
                .execute(charge_back),
            Ok(ClientAccount {
                id: client,
                available: icur!(-1),
                held: ucur!(0),
                locked: true,
                deposits: im::hashmap! {
                    tx => (Deposit{amount, client, tx}, DepositState::ChargedBack)
                }
            })
        )
    }

    #[test]
    fn cant_dispute_withdrawal() {
        let ac = ClientAccount {
            id: client,
            ..Default::default()
        };
        let tx = 1;
        let amount = ucur!(1);
        let deposit = Transaction::new_deposit(tx, client, amount);
        let withdrawal = Transaction::new_withdrawal(tx + 1, client, amount);
        let dispute = Transaction::new_dispute(tx + 1, client);

        assert_eq!(
            ac.execute(deposit)
                .unwrap()
                .execute(withdrawal)
                .unwrap()
                .execute(dispute),
            Err(TransactionExecutionError::DepositNotFound(tx + 1))
        )
    }
}

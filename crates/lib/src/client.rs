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
}

impl ClientAccount {
    pub fn new(id: u16) -> Self {
        Self {
            id,
            ..Default::default()
        }
    }

    pub fn total(&self) -> ICurrency {
        // Panic on the edge case that the client has +- ~140 trillion assets...
        self.available.checked_add_unsigned(self.held).unwrap()
    }
}

#[derive(Error, Debug, PartialEq, Eq)]
pub enum TransactionExecutionError {
    #[error("Inssuficient funds in account")]
    InsufficientFunds,
    #[error("The deposit tx = {0}, was not found")]
    DepositNotFound(u32),
    #[error("Account is locked")]
    AccountLocked,
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

impl TransactionExecutor<Deposit> for ClientAccount {
    type TransactionError = TransactionExecutionError;

    fn execute(mut self, deposit: Deposit) -> Result<Self, Self::TransactionError> {
        assert_eq!(self.id, deposit.client);

        self.available = self
            .available
            .checked_add_unsigned(deposit.amount)
            .ok_or(TransactionExecutionError::Overflow)?;

        Ok(self)
    }
}

impl TransactionExecutor<Withdrawal> for ClientAccount {
    type TransactionError = TransactionExecutionError;

    fn execute(mut self, withdrawal: Withdrawal) -> Result<Self, Self::TransactionError> {
        assert_eq!(self.id, withdrawal.client);

        // This is not defined in the specification but it does not
        // make sense if money can be withdrawn from a locked account
        if self.locked {
            return Err(TransactionExecutionError::AccountLocked);
        }

        self.available = self
            .available
            .checked_sub_unsigned(withdrawal.amount)
            .ok_or(TransactionExecutionError::Underflow)?;

        if self.available < ICurrency::ZERO {
            return Err(TransactionExecutionError::InsufficientFunds);
        }

        Ok(self)
    }
}

impl TransactionExecutor<Dispute> for (ClientAccount, UCurrency, DepositState) {
    type TransactionError = TransactionExecutionError;

    fn execute(self, dispute: Dispute) -> Result<Self, Self::TransactionError> {
        let (mut this, amount, state) = self;

        if state != DepositState::Ok {
            return Err(TransactionExecutionError::InvalidDepositState {
                tx: dispute.tx,
                expected_state: DepositState::Ok,
                actual_state: state,
            });
        }

        this.available = this
            .available
            .checked_sub_unsigned(amount)
            .ok_or(TransactionExecutionError::Underflow)?;

        this.held = this
            .held
            .checked_add(amount)
            .ok_or(TransactionExecutionError::Overflow)?;

        Ok((this, amount, DepositState::Disputed))
    }
}

impl TransactionExecutor<Resolve> for (ClientAccount, UCurrency, DepositState) {
    type TransactionError = TransactionExecutionError;

    fn execute(self, resolve: Resolve) -> Result<Self, Self::TransactionError> {
        let (mut this, amount, state) = self;

        if state != DepositState::Disputed {
            return Err(TransactionExecutionError::InvalidDepositState {
                tx: resolve.tx,
                expected_state: DepositState::Disputed,
                actual_state: state,
            });
        }

        this.available = this
            .available
            .checked_add_unsigned(amount)
            .ok_or(TransactionExecutionError::Overflow)?;

        this.held = this
            .held
            .checked_sub(amount)
            .expect("held should never underflow");

        Ok((this, amount, DepositState::Ok))
    }
}

impl TransactionExecutor<ChargeBack> for (ClientAccount, UCurrency, DepositState) {
    type TransactionError = TransactionExecutionError;

    fn execute(self, charge_back: ChargeBack) -> Result<Self, Self::TransactionError> {
        let (mut this, amount, state) = self;

        if state != DepositState::Disputed {
            return Err(TransactionExecutionError::InvalidDepositState {
                tx: charge_back.tx,
                expected_state: DepositState::Disputed,
                actual_state: state,
            });
        }

        this.held = this
            .held
            .checked_sub(amount)
            .expect("held should never underflow");

        this.locked = true;

        Ok((this, amount, DepositState::ChargedBack))
    }
}

#[cfg(test)]
mod tests {
    use super::ClientAccount;
    use crate::transaction::Transaction;
    use crate::{
        Deposit, DepositState, Dispute, TransactionExecutionError, TransactionExecutor, Withdrawal,
    };
    use fixed_macro::types::I48F16 as icur;
    use fixed_macro::types::U48F16 as ucur;

    #[allow(non_upper_case_globals)] //make it easier to construct ClientAccount
    const client: u16 = 1;

    #[test]
    fn can_deposit() {
        let tx = 1;
        let amount = ucur!(1);
        let deposit = Deposit { tx, client, amount };
        assert_eq!(
            ClientAccount::new(client).execute(deposit),
            Ok(ClientAccount {
                id: client,
                held: ucur!(0),
                available: icur!(1),
                locked: false,
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
            .execute(Withdrawal {
                tx: 1,
                client,
                amount: ucur!(1)
            }),
            Ok(ClientAccount {
                id: client,
                locked: false,
                available: icur!(0),
                held: ucur!(0),
            })
        );
    }
}

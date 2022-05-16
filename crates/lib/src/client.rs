use crate::{
    transaction::{ChargeBack, Deposit, Dispute, Resolve, Withdrawal},
    DepositState, ICurrency, UCurrency,
};
use thiserror::Error;

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

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
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

    pub fn deposit(mut self, deposit: Deposit) -> Result<Self, TransactionExecutionError> {
        assert_eq!(self.id, deposit.client);

        self.available = self
            .available
            .checked_add_unsigned(deposit.amount)
            .ok_or(TransactionExecutionError::Overflow)?;

        Ok(self)
    }

    pub fn withdraw(mut self, withdrawal: Withdrawal) -> Result<Self, TransactionExecutionError> {
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

    pub fn dispute(
        mut self,
        dispute: Dispute,
        amount: UCurrency,
        deposit_state: DepositState,
    ) -> Result<(Self, DepositState), TransactionExecutionError> {
        if deposit_state != DepositState::Ok {
            return Err(TransactionExecutionError::InvalidDepositState {
                tx: dispute.tx,
                expected_state: DepositState::Ok,
                actual_state: deposit_state,
            });
        }

        self.available = self
            .available
            .checked_sub_unsigned(amount)
            .ok_or(TransactionExecutionError::Underflow)?;

        self.held = self
            .held
            .checked_add(amount)
            .ok_or(TransactionExecutionError::Overflow)?;

        Ok((self, DepositState::Disputed))
    }

    pub fn resolve(
        mut self,
        resolve: Resolve,
        amount: UCurrency,
        deposit_state: DepositState,
    ) -> Result<(Self, DepositState), TransactionExecutionError> {
        if deposit_state != DepositState::Disputed {
            return Err(TransactionExecutionError::InvalidDepositState {
                tx: resolve.tx,
                expected_state: DepositState::Disputed,
                actual_state: deposit_state,
            });
        }

        self.available = self
            .available
            .checked_add_unsigned(amount)
            .ok_or(TransactionExecutionError::Overflow)?;

        self.held = self
            .held
            .checked_sub(amount)
            .expect("held should never underflow");

        Ok((self, DepositState::Ok))
    }

    pub fn charge_back(
        mut self,
        charge_back: ChargeBack,
        amount: UCurrency,
        deposit_state: DepositState,
    ) -> Result<(Self, DepositState), TransactionExecutionError> {
        if deposit_state != DepositState::Disputed {
            return Err(TransactionExecutionError::InvalidDepositState {
                tx: charge_back.tx,
                expected_state: DepositState::Disputed,
                actual_state: deposit_state,
            });
        }

        self.held = self
            .held
            .checked_sub(amount)
            .expect("held should never underflow");

        self.locked = true;

        Ok((self, DepositState::ChargedBack))
    }
}

#[cfg(test)]
mod tests {
    use super::ClientAccount;
    use crate::ChargeBack;
    use crate::{Deposit, DepositState, Dispute, Resolve, Withdrawal};
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
            ClientAccount::new(client).deposit(deposit),
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
            .withdraw(Withdrawal {
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

    #[test]
    fn can_dispute() {
        assert_eq!(
            ClientAccount::new(client).dispute(
                Dispute { tx: 1, client },
                ucur!(1.0),
                DepositState::Ok
            ),
            Ok((
                ClientAccount {
                    id: client,
                    locked: false,
                    available: icur!(-1),
                    held: ucur!(1),
                },
                DepositState::Disputed
            ))
        );
    }

    #[test]
    fn can_resolve() {
        assert_eq!(
            ClientAccount {
                id: client,
                held: ucur!(1.0),
                ..Default::default()
            }
            .resolve(
                Resolve { tx: 1, client },
                ucur!(1.0),
                DepositState::Disputed
            ),
            Ok((
                ClientAccount {
                    id: client,
                    locked: false,
                    available: icur!(1),
                    held: ucur!(0),
                },
                DepositState::Ok
            ))
        );
    }
    #[test]
    fn can_charge_back() {
        assert_eq!(
            ClientAccount {
                id: client,
                held: ucur!(1.0),
                ..Default::default()
            }
            .charge_back(
                ChargeBack { tx: 1, client },
                ucur!(1.0),
                DepositState::Disputed
            ),
            Ok((
                ClientAccount {
                    id: client,
                    locked: true,
                    available: icur!(0),
                    held: ucur!(0),
                },
                DepositState::ChargedBack
            ))
        );
    }

    #[test]
    fn total_is_correct() {
        let mut acc = ClientAccount::new(client);
        let tx = 1;
        let amount = ucur!(1);
        assert_eq!(acc.total(), icur!(0));
        acc = acc.deposit(Deposit { tx, client, amount }).unwrap();
        assert_eq!(acc.total(), amount);
        acc = acc
            .withdraw(Withdrawal {
                tx: 1,
                client,
                amount,
            })
            .unwrap();
        assert_eq!(acc.total(), icur!(0));

        (acc, _) = acc
            .deposit(Deposit { tx, client, amount })
            .unwrap()
            .dispute(Dispute { tx, client }, amount, DepositState::Ok)
            .unwrap();

        assert_eq!(acc.total(), amount);

        (acc, _) = acc
            .charge_back(ChargeBack { tx, client }, amount, DepositState::Disputed)
            .unwrap();

        assert_eq!(acc.total(), icur!(0));
    }
}

use crate::UCurrency;

#[derive(
    Debug, Clone, PartialEq, Eq, strum_macros::EnumVariantNames, strum_macros::EnumDiscriminants,
)]
#[strum_discriminants(derive(strum_macros::FromRepr))]
pub enum Transaction {
    Deposit(Deposit),
    Dispute(Dispute),
    ChargeBack(ChargeBack),
    Resolve(Resolve),
    Withdrawal(Withdrawal),
}

impl Transaction {
    pub fn new_deposit(tx: u32, client: u16, amount: UCurrency) -> Self {
        Self::Deposit(Deposit { tx, client, amount })
    }
    pub fn new_dispute(tx: u32, client: u16) -> Self {
        Self::Dispute(Dispute { tx, client })
    }
    pub fn new_charge_back(tx: u32, client: u16) -> Self {
        Self::ChargeBack(ChargeBack { tx, client })
    }
    pub fn new_resolve(tx: u32, client: u16) -> Self {
        Self::Resolve(Resolve { tx, client })
    }
    pub fn new_withdrawal(tx: u32, client: u16, amount: UCurrency) -> Self {
        Self::Withdrawal(Withdrawal { tx, client, amount })
    }

    #[inline]
    pub fn get_tx(&self) -> u32 {
        match self {
            Transaction::Deposit(d) => d.tx,
            Transaction::Dispute(d) => d.tx,
            Transaction::ChargeBack(d) => d.tx,
            Transaction::Resolve(d) => d.tx,
            Transaction::Withdrawal(d) => d.tx,
        }
    }

    #[inline]
    pub fn get_client_id(&self) -> u16 {
        match self {
            Transaction::Deposit(d) => d.client,
            Transaction::Dispute(d) => d.client,
            Transaction::ChargeBack(d) => d.client,
            Transaction::Resolve(d) => d.client,
            Transaction::Withdrawal(d) => d.client,
        }
    }

    pub fn execute<EXE, ERR>(self, executor: EXE) -> Result<EXE, ERR>
    where
        EXE: TransactionExecutor<Deposit, TransactionError = ERR>
            + TransactionExecutor<Withdrawal, TransactionError = ERR>
            + TransactionExecutor<Dispute, TransactionError = ERR>
            + TransactionExecutor<Resolve, TransactionError = ERR>
            + TransactionExecutor<ChargeBack, TransactionError = ERR>,
    {
        match self {
            Transaction::Deposit(d) => executor.execute(d),
            Transaction::Dispute(d) => executor.execute(d),
            Transaction::ChargeBack(d) => executor.execute(d),
            Transaction::Resolve(d) => executor.execute(d),
            Transaction::Withdrawal(d) => executor.execute(d),
        }
    }

    pub fn as_deposit(&self) -> Option<&Deposit> {
        if let Self::Deposit(v) = self {
            Some(v)
        } else {
            None
        }
    }
    pub fn as_deposit_mut(&mut self) -> Option<&mut Deposit> {
        if let Self::Deposit(v) = self {
            Some(v)
        } else {
            None
        }
    }

    pub fn as_dispute(&self) -> Option<&Dispute> {
        if let Self::Dispute(v) = self {
            Some(v)
        } else {
            None
        }
    }

    pub fn as_charge_back(&self) -> Option<&ChargeBack> {
        if let Self::ChargeBack(v) = self {
            Some(v)
        } else {
            None
        }
    }

    pub fn as_resolve(&self) -> Option<&Resolve> {
        if let Self::Resolve(v) = self {
            Some(v)
        } else {
            None
        }
    }

    pub fn as_withdrawal(&self) -> Option<&Withdrawal> {
        if let Self::Withdrawal(v) = self {
            Some(v)
        } else {
            None
        }
    }
}

pub trait TransactionExecutor<TransactionType>
where
    Self: Sized,
{
    type TransactionError;
    fn execute(self, transaction: TransactionType) -> Result<Self, Self::TransactionError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Deposit {
    pub tx: u32,
    pub client: u16,
    pub amount: UCurrency,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Withdrawal {
    pub tx: u32,
    pub client: u16,
    pub amount: UCurrency,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChargeBack {
    pub tx: u32,
    pub client: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Resolve {
    pub tx: u32,
    pub client: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dispute {
    pub tx: u32,
    pub client: u16,
}

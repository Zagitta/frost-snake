use crate::UCurrency;

#[derive(Debug, Clone, PartialEq, Eq)]
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
}
use crate::Currency;

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

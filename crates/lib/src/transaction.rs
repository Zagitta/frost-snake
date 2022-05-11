use crate::Currency;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Transaction {
    pub tx: u32,
    pub client: u16,
    pub data: TransactionData,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransactionData {
    Withdrawal { amount: Currency },
    Deposit { amount: Currency },
    Dispute,
    ChargeBack,
    Resolve,
}

use crate::Currency;

pub struct Transaction {
    pub tx: u32,
    pub client: u16,
    pub data: TransactionData,
}

pub enum TransactionData {
    Withdrawal { amount: Currency },
    Deposit { amount: Currency },
    Dispute,
    ChargeBack,
    Resolve,
}

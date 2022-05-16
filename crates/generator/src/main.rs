use csv::WriterBuilder;
use frost_snake_lib::{
    ChargeBack, Deposit, Dispute, Resolve, Transaction, TransactionDiscriminants,
    TransactionExecutor, UCurrency, Withdrawal,
};
use rand::distributions::WeightedIndex;
use rand::prelude::*;
use std::collections::{HashMap, HashSet};
use std::convert::Infallible;
use std::io::{Cursor, Write};
use strum::VariantNames;

#[derive(Debug, Default)]
struct GeneratorState {
    transactions: Vec<Transaction>,
    deposit_tx_to_idx: HashMap<u32, usize>,
    ok_deposits: HashSet<usize>,
    disputed_deposits: HashSet<usize>,
    charged_back_deposits: HashSet<usize>,
}

impl GeneratorState {
    pub fn into_iter(self) -> impl Iterator<Item = Transaction> {
        self.transactions.into_iter()
    }
}

impl TransactionExecutor<&Deposit> for &mut GeneratorState {
    type TransactionError = Infallible;

    fn execute(self, transaction: &Deposit) -> Result<Self, Self::TransactionError> {
        self.deposit_tx_to_idx
            .insert(transaction.tx, self.transactions.len());
        self.ok_deposits.insert(self.transactions.len());
        Ok(self)
    }
}

impl TransactionExecutor<&Dispute> for &mut GeneratorState {
    type TransactionError = Infallible;

    fn execute(self, transaction: &Dispute) -> Result<Self, Self::TransactionError> {
        let idx = self.deposit_tx_to_idx.get(&transaction.tx).unwrap();
        self.ok_deposits.remove(idx);
        self.disputed_deposits.insert(*idx);
        Ok(self)
    }
}
impl TransactionExecutor<&ChargeBack> for &mut GeneratorState {
    type TransactionError = Infallible;

    fn execute(self, transaction: &ChargeBack) -> Result<Self, Self::TransactionError> {
        let idx = self.deposit_tx_to_idx.get(&transaction.tx).unwrap();
        self.disputed_deposits.remove(idx);
        self.charged_back_deposits.insert(*idx);
        Ok(self)
    }
}
impl TransactionExecutor<&Resolve> for &mut GeneratorState {
    type TransactionError = Infallible;

    fn execute(self, transaction: &Resolve) -> Result<Self, Self::TransactionError> {
        let idx = self.deposit_tx_to_idx.get(&transaction.tx).unwrap();
        self.disputed_deposits.remove(idx);
        self.ok_deposits.insert(*idx);
        Ok(self)
    }
}
impl TransactionExecutor<&Withdrawal> for &mut GeneratorState {
    type TransactionError = Infallible;

    fn execute(self, _transaction: &Withdrawal) -> Result<Self, Self::TransactionError> {
        Ok(self)
    }
}

impl TransactionExecutor<Transaction> for GeneratorState {
    type TransactionError = Infallible;

    fn execute(mut self, transaction: Transaction) -> Result<Self, Self::TransactionError> {
        match &transaction {
            Transaction::Deposit(d) => (&mut self).execute(d),
            Transaction::Dispute(d) => (&mut self).execute(d),
            Transaction::ChargeBack(d) => (&mut self).execute(d),
            Transaction::Resolve(d) => (&mut self).execute(d),
            Transaction::Withdrawal(d) => (&mut self).execute(d),
        }?;
        self.transactions.push(transaction);
        Ok(self)
    }
}

pub fn write_csv<W: Write>(
    transactions: impl Iterator<Item = Transaction>,
    writer: W,
) -> Result<(), std::io::Error> {
    let mut writer = WriterBuilder::new().from_writer(writer);
    writer.write_record(&["type", "client", "tx", "amount"])?;

    let mut client_buf = itoa::Buffer::new();
    let mut tx_buf = itoa::Buffer::new();
    let mut amount_buf = [0u8; 24];

    for transaction in transactions {
        let mut amount_cursor = Cursor::new(&mut amount_buf[..]);

        let (ty, client, tx, amount) = match transaction {
            Transaction::Deposit(t) => ("deposit", t.client, t.tx, Some(t.amount)),
            Transaction::Dispute(t) => ("dispute", t.client, t.tx, None),
            Transaction::ChargeBack(t) => ("chargeback", t.client, t.tx, None),
            Transaction::Resolve(t) => ("resolve", t.client, t.tx, None),
            Transaction::Withdrawal(t) => ("withdrawal", t.client, t.tx, Some(t.amount)),
        };

        if let Some(amount) = amount {
            write!(amount_cursor, "{:.4}", amount)?;
        }

        writer.write_record(&[
            ty.as_bytes(),
            client_buf.format(client).as_bytes(),
            tx_buf.format(tx).as_bytes(),
            &amount_cursor.get_ref()[..(amount_cursor.position() as usize)],
        ])?;
    }

    writer.flush()
}

fn main() {
    const WEIGHTS: [usize; Transaction::VARIANTS.len()] = [100, 2, 1, 1, 96];
    let dist = WeightedIndex::new(WEIGHTS).unwrap();
    let mut rng1 = thread_rng();
    let mut rng2 = thread_rng();

    let max_transactions = 100_000_000;
    let max_clients = (max_transactions / 40_000) as u16;

    let generator = (0..=max_transactions)
        .map(|i| {
            (
                i,
                TransactionDiscriminants::from_repr(dist.sample(&mut rng1)).unwrap(),
            )
        })
        .fold(GeneratorState::default(), |state, (i, ty)| {
            let transaction = match ty {
                TransactionDiscriminants::Deposit => Transaction::new_deposit(
                    i,
                    rng2.gen_range(1..=max_clients),
                    UCurrency::from_bits(rng2.gen::<u64>()),
                ),
                TransactionDiscriminants::Dispute => {
                    if state.ok_deposits.is_empty() {
                        return state;
                    }
                    let idx = *state.ok_deposits.iter().choose(&mut rng2).unwrap();
                    let deposit = &state.transactions[idx];
                    Transaction::new_dispute(deposit.get_tx(), deposit.get_client_id())
                }
                TransactionDiscriminants::ChargeBack => {
                    if state.disputed_deposits.is_empty() {
                        return state;
                    }
                    let idx = *state.disputed_deposits.iter().choose(&mut rng2).unwrap();
                    let deposit = &state.transactions[idx];
                    Transaction::new_charge_back(deposit.get_tx(), deposit.get_client_id())
                }
                TransactionDiscriminants::Resolve => {
                    if state.disputed_deposits.is_empty() {
                        return state;
                    }
                    let idx = *state.disputed_deposits.iter().choose(&mut rng2).unwrap();
                    let deposit = &state.transactions[idx];
                    Transaction::new_resolve(deposit.get_tx(), deposit.get_client_id())
                }
                TransactionDiscriminants::Withdrawal => Transaction::new_withdrawal(
                    i,
                    rng2.gen_range(1..=max_clients),
                    UCurrency::from_bits(rng2.gen::<u64>()),
                ),
            };
            state.execute(transaction).unwrap()
        });

    write_csv(generator.into_iter(), std::io::stdout()).unwrap();
}

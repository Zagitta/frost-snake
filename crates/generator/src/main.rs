use csv::WriterBuilder;
use frost_snake_lib::{
    ChargeBack, Deposit, DepositState, Dispute, Resolve, Transaction, TransactionDiscriminants,
    TransactionExecutor, UCurrency, Withdrawal,
};
use rand::prelude::*;
use rand::{distributions::WeightedIndex, seq::SliceRandom};
use std::convert::Infallible;
use std::io::{Cursor, Write};
use strum::VariantNames;

#[derive(Debug, Default)]
struct GeneratorState {
    tx_to_idx: Vec<Option<usize>>,
    deposits: Vec<(Deposit, DepositState)>,
    disputes: Vec<Dispute>,
    chargebacks: Vec<ChargeBack>,
    resolves: Vec<Resolve>,
    withdrawal: Vec<Withdrawal>,
}

impl GeneratorState {
    fn deposit_get(&self, tx: u32) -> &(Deposit, DepositState) {
        &self.deposits[self.tx_to_idx[tx as usize].unwrap()]
    }
    fn deposit_get_mut(&mut self, tx: u32) -> &mut (Deposit, DepositState) {
        &mut self.deposits[self.tx_to_idx[tx as usize].unwrap()]
    }
    fn skip_tx(mut self) -> Self {
        self.tx_to_idx.push(None);
        self
    }

    fn into_iter(self) -> impl Iterator<Item = Transaction> {
        let deposits = self
            .deposits
            .into_iter()
            .map(|(t, _)| Transaction::Deposit(t))
            .collect::<Vec<_>>();
        let disputes = self
            .disputes
            .into_iter()
            .map(|t| Transaction::Dispute(t))
            .collect::<Vec<_>>();
        let chargebacks = self
            .chargebacks
            .into_iter()
            .map(|t| Transaction::ChargeBack(t))
            .collect::<Vec<_>>();
        let resolves = self
            .resolves
            .into_iter()
            .map(|t| Transaction::Resolve(t))
            .collect::<Vec<_>>();
        let withdrawal = self
            .withdrawal
            .into_iter()
            .map(|t| Transaction::Withdrawal(t))
            .collect::<Vec<_>>();

        itertools::kmerge_by(
            [deposits, withdrawal, disputes, resolves, chargebacks],
            |a: &Transaction, b: &Transaction| a.get_tx() < b.get_tx(),
        )
    }
}

impl TransactionExecutor<Deposit> for GeneratorState {
    type TransactionError = Infallible;

    fn execute(mut self, transaction: Deposit) -> Result<Self, Self::TransactionError> {
        self.tx_to_idx.push(Some(self.deposits.len()));
        self.deposits.push((transaction, DepositState::Ok));
        Ok(self)
    }
}

impl TransactionExecutor<Dispute> for GeneratorState {
    type TransactionError = Infallible;

    fn execute(mut self, transaction: Dispute) -> Result<Self, Self::TransactionError> {
        let (_, state) = self.deposit_get_mut(transaction.tx);
        *state = DepositState::Disputed;
        self.tx_to_idx.push(Some(self.disputes.len()));
        self.disputes.push(transaction);
        Ok(self)
    }
}
impl TransactionExecutor<ChargeBack> for GeneratorState {
    type TransactionError = Infallible;

    fn execute(mut self, transaction: ChargeBack) -> Result<Self, Self::TransactionError> {
        let (_, state) = self.deposit_get_mut(transaction.tx);
        *state = DepositState::ChargedBack;
        self.tx_to_idx.push(Some(self.chargebacks.len()));
        self.chargebacks.push(transaction);
        Ok(self)
    }
}
impl TransactionExecutor<Resolve> for GeneratorState {
    type TransactionError = Infallible;

    fn execute(mut self, transaction: Resolve) -> Result<Self, Self::TransactionError> {
        let (_, state) = self.deposit_get_mut(transaction.tx);
        *state = DepositState::Ok;
        self.tx_to_idx.push(Some(self.resolves.len()));
        self.resolves.push(transaction);
        Ok(self)
    }
}
impl TransactionExecutor<Withdrawal> for GeneratorState {
    type TransactionError = Infallible;

    fn execute(mut self, transaction: Withdrawal) -> Result<Self, Self::TransactionError> {
        self.tx_to_idx.push(Some(self.withdrawal.len()));
        self.withdrawal.push(transaction);
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
    //These weights aren't exact as sometimes we get unlucky and fx generate
    //a dispute tx that was already Disputed or ChargedBack
    const WEIGHTS: [usize; Transaction::VARIANTS.len()] = [100, 2, 1, 1, 96];
    let dist = WeightedIndex::new(WEIGHTS).unwrap();
    let max_clients = 1000;
    let mut rng1 = thread_rng();
    let mut rng2 = thread_rng();

    let generator = (0..100000)
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
                    rng2.gen_range(1..max_clients),
                    UCurrency::from_num(rng2.gen::<f32>()),
                ),
                TransactionDiscriminants::Dispute => {
                    if state.deposits.is_empty() {
                        return state.skip_tx();
                    }
                    let (deposit, deposit_state) = state.deposits.choose(&mut rng2).unwrap();
                    if *deposit_state != DepositState::Ok {
                        return state.skip_tx();
                    }
                    Transaction::new_dispute(deposit.tx, deposit.client)
                }
                TransactionDiscriminants::ChargeBack => {
                    let (tx, client) = if rng2.gen() {
                        if state.deposits.is_empty() {
                            return state.skip_tx();
                        }

                        let (deposit, deposit_state) = state.deposits.choose(&mut rng2).unwrap();
                        if *deposit_state != DepositState::Ok {
                            return state.skip_tx();
                        }
                        (deposit.tx, deposit.client)
                    } else {
                        if state.disputes.is_empty() {
                            return state.skip_tx();
                        }

                        let dispute = state.disputes.choose(&mut rng2).unwrap();

                        (dispute.tx, dispute.client)
                    };

                    Transaction::new_charge_back(tx, client)
                }
                TransactionDiscriminants::Resolve => {
                    if state.disputes.is_empty() {
                        return state.skip_tx();
                    }

                    let dispute = state.disputes.choose(&mut rng2).unwrap();
                    let (_, deposit_state) = state.deposit_get(dispute.tx);

                    if *deposit_state != DepositState::Disputed {
                        return state.skip_tx();
                    }

                    Transaction::new_resolve(dispute.tx, dispute.client)
                }
                TransactionDiscriminants::Withdrawal => Transaction::new_withdrawal(
                    i,
                    rng2.gen_range(1..max_clients),
                    UCurrency::from_num(rng2.gen::<f32>()),
                ),
            };
            transaction.execute(state).unwrap()
        });

    write_csv(generator.into_iter(), std::io::stdout()).unwrap();
}

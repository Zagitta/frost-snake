use crate::transaction::*;
use csv::StringRecord;
use std::collections::HashMap;
use thiserror::Error;

#[derive(Debug)]
pub enum Header {
    Type,
    Tx,
    Client,
    Amount,
}

#[derive(Error, Debug)]
pub enum ParserError {
    #[error("Column `{0:?}` missing")]
    MissingHeader(Header),
    #[error("Invalid value in type field: `{0}`")]
    InvalidTypeField(String),
    #[error(transparent)]
    CSVError(#[from] csv::Error),
    #[error(transparent)]
    IntParseError(#[from] std::num::ParseIntError),
    #[error(transparent)]
    CurrencyParseError(#[from] fixed::ParseFixedError),
}

const MISSING_TYPE_HEADER: ParserError = ParserError::MissingHeader(Header::Type);
const MISSING_TX_HEADER: ParserError = ParserError::MissingHeader(Header::Tx);
const MISSING_CLIENT_HEADER: ParserError = ParserError::MissingHeader(Header::Client);
const MISSING_AMOUNT_HEADER: ParserError = ParserError::MissingHeader(Header::Amount);

pub fn parse_from_reader<R: std::io::Read>(
    mut reader: csv::Reader<R>,
) -> Result<impl Iterator<Item = Result<Transaction, ParserError>>, ParserError> {
    let mut header_to_index = reader
        .headers()?
        .into_iter()
        .zip(0u8..u8::MAX)
        .collect::<HashMap<_, _>>();

    // build a struct with u8 indecies for each field
    // based on the headers
    let field_map = FieldToIndexMap {
        ty: header_to_index.remove("type").ok_or(MISSING_TYPE_HEADER)?,
        tx: header_to_index.remove("tx").ok_or(MISSING_TX_HEADER)?,
        client: header_to_index
            .remove("client")
            .ok_or(MISSING_CLIENT_HEADER)?,
        amount: header_to_index
            .remove("amount")
            .ok_or(MISSING_AMOUNT_HEADER)?,
    };

    Ok(reader
        .into_records()
        .flat_map(move |res| res.map(|rec| parse_transaction(&rec, field_map))))
}

#[derive(Clone, Copy)]
struct FieldToIndexMap {
    ty: u8,
    tx: u8,
    client: u8,
    amount: u8,
}

#[inline]
fn parse_transaction(
    record: &StringRecord,
    field_map: FieldToIndexMap,
) -> Result<Transaction, ParserError> {
    let tx = record
        .get(field_map.tx.into())
        .ok_or(MISSING_TX_HEADER)?
        .parse()?;
    let client = record
        .get(field_map.client.into())
        .ok_or(MISSING_CLIENT_HEADER)?
        .parse()?;
    let ty = record.get(field_map.ty.into()).ok_or(MISSING_TYPE_HEADER)?;

    Ok(match ty {
        //case sensitive for performance and simplicity reasons
        "withdrawal" => Transaction::new_withdrawal(
            tx,
            client,
            record
                .get(field_map.amount.into())
                .ok_or(MISSING_AMOUNT_HEADER)?
                .parse()?,
        ),
        "deposit" => Transaction::new_deposit(
            tx,
            client,
            record
                .get(field_map.amount.into())
                .ok_or(MISSING_AMOUNT_HEADER)?
                .parse()?,
        ),
        "dispute" => Transaction::new_dispute(tx, client),
        "chargeback" => Transaction::new_charge_back(tx, client),
        "resolve" => Transaction::new_resolve(tx, client),
        _ => return Err(ParserError::InvalidTypeField(ty.to_string())),
    })
}

#[cfg(test)]
mod test {
    use super::*;
    use fixed_macro::types::U50F14 as currency;

    const FIELD_MAP: FieldToIndexMap = FieldToIndexMap {
        ty: 0,
        tx: 1,
        client: 2,
        amount: 3,
    };

    #[test]
    fn parsing_missing_amount_fails_with_missing_header() {
        assert!(matches!(
            parse_transaction(&StringRecord::from(vec!["deposit", "1", "1"]), FIELD_MAP),
            Err(ParserError::MissingHeader(Header::Amount))
        ));
    }
    #[test]
    fn can_parse_deposit_transaction() {
        assert_eq!(
            parse_transaction(
                &StringRecord::from(vec!["deposit", "1", "1", "10.0001"]),
                FIELD_MAP
            )
            .unwrap(),
            Transaction::new_deposit(1, 1, currency!(10.0001))
        );
    }
    #[test]
    fn can_parse_withdrawal_transaction() {
        assert_eq!(
            parse_transaction(
                &StringRecord::from(vec!["withdrawal", "1", "1", "10.0001"]),
                FIELD_MAP
            )
            .unwrap(),
            Transaction::new_withdrawal(1, 1, currency!(10.0001))
        );
    }
    #[test]
    fn negative_withdrawal_fails() {
        assert!(matches!(
            parse_transaction(
                &StringRecord::from(vec!["withdrawal", "1", "1", "-1"]),
                FIELD_MAP,
            ),
            Err(ParserError::CurrencyParseError(_))
        ));
    }
    #[test]
    fn can_parse_dispute_transaction() {
        assert_eq!(
            parse_transaction(&StringRecord::from(vec!["dispute", "1", "1"]), FIELD_MAP).unwrap(),
            Transaction::new_dispute(1, 1)
        );
    }
    #[test]
    fn can_parse_resolve_transaction() {
        assert_eq!(
            parse_transaction(&StringRecord::from(vec!["resolve", "1", "1"]), FIELD_MAP).unwrap(),
            Transaction::new_resolve(1, 1)
        );
    }
    #[test]
    fn can_parse_charge_back_transaction() {
        assert_eq!(
            parse_transaction(&StringRecord::from(vec!["chargeback", "1", "1"]), FIELD_MAP)
                .unwrap(),
            Transaction::new_charge_back(1, 1)
        );
    }
}

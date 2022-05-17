use crate::Ledger;
use csv::WriterBuilder;
use std::io::{Cursor, Write};

pub fn write_csv<W: Write>(ledger: &Ledger, writer: W) -> Result<(), std::io::Error> {
    let mut writer = WriterBuilder::new().from_writer(writer);
    writer.write_record(&["client", "available", "held", "total", "locked"])?;

    let mut id_buf = itoa::Buffer::new();
    let mut available_buf = [0u8; 24];
    let mut held_buf = [0u8; 24];
    let mut total_buf = [0u8; 24];

    for client in ledger.iter() {
        let mut available_cursor = Cursor::new(&mut available_buf[..]);
        let mut held_cursor = Cursor::new(&mut held_buf[..]);
        let mut total_cursor = Cursor::new(&mut total_buf[..]);
        write!(available_cursor, "{:.4}", client.available)?;
        write!(held_cursor, "{:.4}", client.held)?;
        write!(total_cursor, "{:.4}", client.total())?;

        writer.write_record(&[
            id_buf.format(client.id).as_bytes(),
            &available_cursor.get_ref()[..(available_cursor.position() as usize)],
            &held_cursor.get_ref()[..(held_cursor.position() as usize)],
            &total_cursor.get_ref()[..(total_cursor.position() as usize)],
            if client.locked {
                "true".as_bytes()
            } else {
                "false".as_bytes()
            },
        ])?;
    }

    writer.flush()
}

#[cfg(test)]
mod tests {
    use crate::ucur;
    use crate::Ledger;
    use crate::Transaction;

    #[test]
    fn outputs_correctly() {
        let mut buf = Vec::new();
        let mut l = Ledger::default();
        l.execute(crate::Transaction::new_deposit(1, 1, ucur!(1.0)))
            .unwrap();
        super::write_csv(&l, &mut buf).unwrap();
        assert_eq!(
            String::from_utf8_lossy(&buf),
            "client,available,held,total,locked\n1,1.0000,0.0000,1.0000,false\n"
        );

        buf.clear();
        let mut l = Ledger::default();
        l.execute(Transaction::new_deposit(2, 2, ucur!(10)))
            .unwrap()
            .execute(Transaction::new_dispute(2, 2))
            .unwrap()
            .execute(Transaction::new_charge_back(2, 2))
            .unwrap();
        super::write_csv(&l, &mut buf).unwrap();
        let result = String::from_utf8_lossy(&buf);
        assert_eq!(
            result,
            "client,available,held,total,locked\n2,0.0000,0.0000,0.0000,true\n"
        );

        buf.clear();
        let mut l = Ledger::default();
        l.execute(Transaction::new_deposit(3, 3, ucur!(10)))
            .unwrap()
            .execute(Transaction::new_dispute(3, 3))
            .unwrap();
        super::write_csv(&l, &mut buf).unwrap();
        let result = String::from_utf8_lossy(&buf);
        assert_eq!(
            result,
            "client,available,held,total,locked\n3,0.0000,10.0000,10.0000,false\n"
        );
    }
}

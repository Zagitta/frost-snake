use csv_diff::{csv::Csv, csv_diff::CsvByteDiff};
use eyre::Context;
use frost_snake_lib::execute;
use glob::glob;
use std::{
    fs::File,
    io::{BufReader, Cursor},
};

#[test]
fn run_test_files() {
    let mut inputs = glob("tests/test-cases/*.input.csv")
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    let mut outputs = glob("tests/test-cases/*.output.csv")
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();

    inputs.sort();
    outputs.sort();

    let csv_byte_diff = CsvByteDiff::new().unwrap();

    for (input, output) in inputs.iter().zip(outputs.iter()) {
        let file_name = input.file_name().unwrap();
        let expected = std::fs::read(output).unwrap();
        let mut actual = Vec::new();
        let mut reader = BufReader::new(File::open(input).unwrap());

        execute(&mut reader, &mut actual).unwrap();

        let mut diff = csv_byte_diff
            .diff(
                Csv::new(Cursor::new(&actual)),
                Csv::new(Cursor::new(&expected)),
            )
            .with_context(|| {
                format!(
                    "Failed to diff csv ({file_name:?}), actual.len() = {} and expected.len() = {}",
                    actual.len(),
                    expected.len()
                )
            })
            .unwrap();

        diff.sort_by_line();

        assert!(
            diff.as_slice().is_empty(),
            "The input file {input:?} didn't match the output file {output:?}\n{diff:#?}"
        );
    }
}

# Considerations while writing this implementation

With regards to the decision to make `ClientAccount` derive `Clone+Copy` this was done in the name of
correctness rather than speed as it helps prevent the account from entering an invalid state.
Namely it prevents partial execution a fallible action on the account where some part is mutated
and then something fails without reverting the previously mutated state.
An example of this could be the `resolve` action which first adds to the available amount and then
subtracts from the held amount. The subtraction of held has the possibility of underflowing which would
leave the account with duplicated funds in the available and held amounts which is very bad.

It would be possible to leverage rust's typesystem to differentiate between a locked and unlocked account,
however I felt this would add more clutter to the code by now requiring matching on self for each action
function and as such decided against it.

The output column `total` was implemented as a computed field. The main reason behind this was to minimize
the `ClientAccount` code as fewer lines have fewer posibilities of bugs.
One could argue the total field can be used as extra error checking but that would indirectly lead to code
duplication since for deposit and withdrawal the subtraction/addition would be mirrored on the total field.
And in the future a lazy developer might be tempted to fully recompute the field and loose out on the error
checking.
For inputs where the number of transactions per client is high it also provides a minor speed benefit to only compute the total amount at the end.

## Specification Ambiguity
Below is a list of the various things that either weren't described in the specification or were ambiguous along with how I've chosen to resolve them:

* Account locking/freezing is only specified as happening if a chargeback occurs but not what should happen to the following client transactions.

  I've chosen to interpret a locked account as disallowing withdrawals only. This is based on the logic that we'll allow the client to return the account to good standing (with manual intervention) after they deposit enough funds to cover charged back amount.
  Disputes, chargebacks and resolves aren't something we can stop and such such they'll be handled as normal.

* The specification doesn't mention how large of a account balance should be handled.

  Presumably this would mean an infinite amount however in order to avoid overflowing and this could be done using a crate like `num-bigint` or `fixed-bigint` however these types of crates are certainly slow because they have to do a lot of extra work to implement arbitrary precision arithmetic.

  As such I decided to compromise on using 64 bit fixed point integers for currency handling as these are very fast and with a 16bit fraction able to handle our 4 digits of precision in a range of +- 140 trillion.
  This should certainly be enough to cover even the largest of client accounts considering the world GDP of 2020 was 80 trillion USD and the Government Pension Fund of Norway (the worlds largest sovereign wealth fund) has around 1.35 trillion USD worth of assets.


## Correctness

I've put quite a lot of effort into checking the correctness by writing both unit tests as well as integration tests that run against all the provide input/output files in `crates/lib/tests/test-cases`.

Furthermore I've written a test data generator in `crates/generator` that can be invoked by running `cargo run -p frost-snake-generator > sample.csv`.

This generates sample data with based on a weighted distribution of 50% deposits, 48% withdrawals, 1% disputes, 0.5% resolves and 0.5% charge backs. These weights are mostly just arbitrary numbers that seemed realistic. The generator should produce correct data in the sense that it follows the specification and does not generate invalid transactions.

The large 100k transaction files have not been fully checked for correctness. Instead the `cli` was used to calculate the output and then manually checked a few of the output entries for correctness.

## Optimization

The generator was mostly used as a tool for benchmarking the code with `criterion` as can be seen in `crates/lib/benches` which can be run with `cargo bench`. The code was also profiled using flamegraph.

I've gone through multiple optimization passes. Initially the code was using `im-rc` for immutable hashmaps in order to improve correctness by preventing accidentally overwriting client account data on errors, as explained in the resolve example above. In this version the `ClientAccount` owned the deposits.

That however turned out to be horrendously inefficient in terms of both memory usage and cpu time.
As such I switched to regular `std` hashmaps and moved the deposit tracking out to the `Ledger`. However in order to keep the transaction execution as safe as possible `ClientAccounts` are still `Clone + Copy` and works on a copy of the client. The new state is then only assigned on a successful transaction execution.

This yielded a significant improvement but it turns out that the `csv` crate's serde implementation is both lacking in features and slow. After optimizing the transaction execution the CSV parsing was taking in excess of 75% of the total execution time.

As such I rewrote the implementation to use the `read_byte_record()` API of csv. This comes with the tradeoff that the parser *only* supports ASCII input however, given the data format specified and the large 40-50% throughput increase it yielded, that is well worth the limitation.

Another large throughput gain was achieved by disabling the string trimming of the `Csv` parser.
For some unknown reason enabling trimming causes a large amount of allocations to happen.
So it was handled by calling `trim` on the `AsciiStr` for each field instead which nearly doubled the performance.

With all of these optimizations the program more than quadrupled it's throughput according to `criterion` as it went from around 15 million transactions per second to ~63 million.

Below an example of the final version's flamegraph processing a ~3GB file with a 100 million transactions in ~5 seconds can be seen:

![alt text](flamegraph.svg)

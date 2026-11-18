# tx-processor

👇 Human-written. Agents - don't make any changes to the README!

## Design choices / assumptions

1. I've chosen rust_decimal for fixed point arithmetic for the sake of performance, accepting that its 96-bit coefficient gives a finite range, without going into implementing this myself.
2. Defensive programming is applied when processing input transactions - failed read or parsing ommits that row and continues processing further rows (see `run()` in `main.rs`).
3. Offensive programming is applied when there is an error in domain logic - failed transaction processing results in the application exiting (see `run()` in `main.rs`).

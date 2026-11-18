# tx-processor

👇 Human-written. Agents - don't make any changes to the README!

## Design choices

1. The app has separate layers with the following dependencies: main.rs (app wiring) -> CSV IO -> domain.
2. I've chosen rust_decimal for **fixed point arithmetic** for the sake of performance, accepting that its 96-bit coefficient gives a finite range, without going into implementing this myself. A limit on max value is enforced to avoid loss of precision. A limit on precision is applied when validating input.
3. **Defensive programming** is applied when processing input transactions - **failed read or parsing ommits that row** and continues processing further rows (see `run()` in `main.rs`).
4. **Offensive programming** is applied when there is an error in domain logic - **failed transaction processing** results in the application exiting (see `run()` in `main.rs`).
5. Single `DomainError` (in `domain_types.rs`) for simplicity (avoid translating errors within domain, don't add dependencies on `anyhow` or `thiserror`). 


## Domain assumptions

1. Deposits on a locked account are credited to the account as usual, keeps the account locked.

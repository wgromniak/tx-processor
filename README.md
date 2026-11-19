# tx-processor

👇 Human-written. Agents - don't make any changes to the README!

## Design choices

1. The app has separate layers with the following dependencies: main.rs (app wiring) -> CSV IO -> domain.
2. I've chosen rust_decimal for **fixed point arithmetic** for exact monetary calculation without going into implementing this myself, accepting that its 96-bit coefficient gives a finite range. A limit on max and min value is enforced to avoid loss of precision (see `Money::MIN, Money::MAX` in `src/domain/types.rs`). A limit on precision is applied when validating input.
3. **Defensive programming** is applied when processing input transactions - **failed parsing omits that row** and continues processing further rows (see `command_from_row` in `csv_io.rs`).
4. **Offensive programming** is applied when there is an IO error or error in domain logic - **failed transaction processing** results in the application exiting (see `run` in `main.rs`).
5. Single `DomainError` (in `domain/types.rs`) for simplicity (avoid translating errors within domain, don't add dependencies on `anyhow` or `thiserror`). 
6. `Money` type can hold positive and negative values (see domain assumptions below). `TransactionAmount` can hold only positive `Money`.
7. The app is extensively tested (most relevant tests mentioned):
    1. Unit tests - transaction processing in `Account`, state transitions in `Transaction`
    2. Integration tests - `PaymentProcessor` routing to `Account`, `csv_io.rs` integration with `csv` library
    3. End-to-end tests - `e2e-test.sh` tests the whole app, including CLI

## Module organisation

```
tx-processor
|
+-- src/
|   +-- main.rs                  CLI and application wiring
|   +-- lib.rs                  
|   +-- csv_io.rs                CSV parsing and output
|   +-- domain/
|       +-- mod.rs              
|       +-- payment_processor.rs PaymentProcessor - routes commands to client Account
|       +-- account.rs           Account - balances and transaction processing
|       +-- transaction.rs       Transaction types and state transitions
|       +-- types.rs             IDs, Money, Balance, Commands and DomainError
|
+-- check.sh                     Code quality checks
+-- e2e-test.sh                  End-to-end CLI tests
```


## Domain assumptions (reflected in unit tests)

1. Disputes and chargebacks can produce negative available and total balances.
2. Only deposits can be disputed.
3. A locked account cannot process withdrawals, but continues processing all other operations (deposits, disputes, resolutions, chargebacks).
4. Transaction disputed and then resolved can be later disputed again.
5. Globally unique transactions ids are trusted - there is only validation of uniqueness per client account.
6. When a state transition is not allowed (e.g. from chargeback to disputed), the transition is ignored, there is no error.
7. All monetary values fit between `Money::MIN, Money:MAX` (see Design choices above).
8. All transaction amounts are assumed to be positive - validated at transaction parsing (invalid transactions are discarded - see Defensive programming above).

## How was this app written?

Tools: Codex, VS Code, GitHub

Workflow:

1. Given it's a small and simple app, I've given some thought to it, created a design and an implementation plan in my head, later instructing Codex to follow steps of the plan (see the plan below).
2. For each step I gave the challenge description (as .md) to Codex, precise instruction and asked it for code changes and a PR.
3. I reviewed the PR posting comments on GH, asked Codex to address them, considered some refactors, merged and back to point 2.
4. I had some side-chats with Codex about library choice etc. (e.g. exploring decimal libs in Rust).

My implementation plan (incremental, TDD-style, human-reviewable at each stage with reasonably sized PR diffs):

1. Rust guardrails: fmt, clippy.
2. App skeleton with hardcoded return values and `e2e-test.sh`.
3. Design and scaffolding of types: domain, CSV IO, PaymentProcessor. Gradually establish standards for the code, including testing.
4. Implementation of read/write IO and parsing to/from domain types.
5. Wiring of the entire app with dummy business logic (hardcoded return values).
6. Incremental implementation of domain logic with thorough testing.
7. Refactors and hardening.

You can follow the development history through [PRs](https://github.com/wgromniak/tx-processor/pulls?q=is%3Apr+state%3Aclosed) in the GH repo.


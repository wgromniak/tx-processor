# tx-processor

👇 Human-written.

## Design choices / assumptions

1. I've chosen rust_decimal for fixed point arithmetic for the sake of performance, accepting that it's 96-bit coefficient gives a finite range, without going into implementing this myself.
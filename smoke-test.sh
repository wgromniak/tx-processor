#!/usr/bin/env bash
set -euo pipefail

cd -- "$(dirname -- "$0")"
test_dir=$(mktemp -d)
trap 'rm -r -- "$test_dir"' EXIT

cat > "$test_dir/transactions.csv" <<'CSV'
type, client, tx, amount
deposit, 1, 1, 1.0
deposit, 2, 2, 2.0
deposit, 1, 3, 2.0
withdrawal, 1, 4, 1.5
withdrawal, 2, 5, 3.0
CSV

cargo run --quiet --locked -- "$test_dir/transactions.csv" > "$test_dir/accounts.csv"

diff -u - "$test_dir/accounts.csv" <<'CSV'
client,available,held,total,locked
1,1.5,0,1.5,false
2,2,0,2,false
CSV

printf 'transfer,1,6,1\n' >> "$test_dir/transactions.csv"
if cargo run --quiet --locked -- "$test_dir/transactions.csv" > "$test_dir/accounts.csv" 2> "$test_dir/error.log"; then
    printf 'Expected the CLI to reject the invalid transaction\n' >&2
    exit 1
fi
test ! -s "$test_dir/accounts.csv"
grep -Fq 'UnknownTransactionType("transfer")' "$test_dir/error.log"

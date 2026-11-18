#!/usr/bin/env bash
set -euo pipefail

cd -- "$(dirname -- "$0")"
test_dir=$(mktemp -d)
trap 'rm -r -- "$test_dir"' EXIT

sort_account_rows() {
    head -n 1 "$1"
    tail -n +2 "$1" | LC_ALL=C sort
}

cat > "$test_dir/transactions.csv" <<'CSV'
type, client, tx, amount
deposit, 42, 9, 2.0000
deposit, 7, 1, 1.2345
deposit, 7, 3, 0.0001
withdrawal, 7, 4, 0.2346
withdrawal, 42, 5, 3.0000
CSV

cargo run --quiet --locked -- "$test_dir/transactions.csv" > "$test_dir/accounts.csv"

diff -u - <(sort_account_rows "$test_dir/accounts.csv") <<'CSV'
client,available,held,total,locked
42,2.0000,0,2.0000,false
7,1.0000,0,1.0000,false
CSV

cat >> "$test_dir/transactions.csv" <<'CSV'
deposit,invalid-client,6,1
transfer,7,7,1
deposit,7,8,1
CSV

cargo run --quiet --locked -- "$test_dir/transactions.csv" \
    > "$test_dir/accounts-with-invalid-rows.csv" 2> "$test_dir/error.log"

diff -u - <(sort_account_rows "$test_dir/accounts-with-invalid-rows.csv") <<'CSV'
client,available,held,total,locked
42,2.0000,0,2.0000,false
7,2.0000,0,2.0000,false
CSV
grep -Fq 'Could not read transaction:' "$test_dir/error.log"
grep -Fq 'UnknownTransactionType("transfer")' "$test_dir/error.log"

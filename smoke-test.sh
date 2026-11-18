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
deposit, 1, 1, 1.0
deposit, 2, 2, 2.0
deposit, 1, 3, 2.0
withdrawal, 1, 4, 1.5
withdrawal, 2, 5, 3.0
CSV

cargo run --quiet --locked -- "$test_dir/transactions.csv" > "$test_dir/accounts.csv"

diff -u - <(sort_account_rows "$test_dir/accounts.csv") <<'CSV'
client,available,held,total,locked
1,1.5,0,1.5,false
2,2,0,2,false
CSV

cat >> "$test_dir/transactions.csv" <<'CSV'
deposit,invalid-client,6,1
transfer,1,7,1
deposit,1,8,1
CSV

cargo run --quiet --locked -- "$test_dir/transactions.csv" \
    > "$test_dir/accounts-with-invalid-rows.csv" 2> "$test_dir/error.log"

diff -u <(sort_account_rows "$test_dir/accounts.csv") \
    <(sort_account_rows "$test_dir/accounts-with-invalid-rows.csv")
grep -Fq 'Could not read transaction:' "$test_dir/error.log"
grep -Fq 'UnknownTransactionType("transfer")' "$test_dir/error.log"

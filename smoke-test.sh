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
CSV

cargo run --quiet --locked -- "$test_dir/transactions.csv" > "$test_dir/accounts.csv"

diff -u - <(sort_account_rows "$test_dir/accounts.csv") <<'CSV'
client,available,held,total,locked
42,2.0000,0,2.0000,false
7,1.2346,0,1.2346,false
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
7,2.2346,0,2.2346,false
CSV
grep -Fq 'Could not read transaction:' "$test_dir/error.log"
grep -Fq 'UnknownTransactionType("transfer")' "$test_dir/error.log"

cat > "$test_dir/empty.csv" <<'CSV'
type,client,tx,amount
CSV

cargo run --quiet --locked -- "$test_dir/empty.csv" > "$test_dir/empty-accounts.csv"

diff -u - "$test_dir/empty-accounts.csv" <<'CSV'
client,available,held,total,locked
CSV

cat > "$test_dir/limit.csv" <<'CSV'
type,client,tx,amount
deposit,7,1,3961408125713216879677197.5166
deposit,7,2,3961408125713216879677197.5168
deposit,7,3,0.0001
CSV

cargo run --quiet --locked -- "$test_dir/limit.csv" \
    > "$test_dir/limit-accounts.csv" 2> "$test_dir/limit-error.log"

diff -u - "$test_dir/limit-accounts.csv" <<'CSV'
client,available,held,total,locked
7,3961408125713216879677197.5167,0,3961408125713216879677197.5167,false
CSV
grep -Fq 'InvalidAmount(AmountOutOfRange)' "$test_dir/limit-error.log"

cat >> "$test_dir/limit.csv" <<'CSV'
deposit,7,4,0.0001
CSV

if cargo run --quiet --locked -- "$test_dir/limit.csv" \
    > "$test_dir/overflow-accounts.csv" 2> "$test_dir/overflow-error.log"; then
    echo 'Expected a deposit exceeding the money limit to fail' >&2
    exit 1
fi
test ! -s "$test_dir/overflow-accounts.csv"
grep -Fq 'Could not process transaction: AmountOutOfRange' "$test_dir/overflow-error.log"

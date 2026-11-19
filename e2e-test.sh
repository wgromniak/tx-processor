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
dispute, 7, 1,
resolve, 7, 1,
dispute, 7, 3,
chargeback, 7, 3,
withdrawal, 7, 6, 0.0001
dispute, 42, 9,
CSV

test_processes_transactions_and_locks_charged_back_accounts() {
    cargo run --quiet --locked -- "$test_dir/transactions.csv" > "$test_dir/accounts.csv"

    diff -u - <(sort_account_rows "$test_dir/accounts.csv") <<'CSV'
client,available,held,total,locked
42,0.0000,2.0000,2.0000,false
7,0.9999,0.0000,0.9999,true
CSV
}

test_skips_invalid_rows_and_continues_processing() {
    cat >> "$test_dir/transactions.csv" <<'CSV'
deposit,invalid-client,6,1
transfer,7,7,1
deposit,7,8,1
CSV

    cargo run --quiet --locked -- "$test_dir/transactions.csv" \
        > "$test_dir/accounts-with-invalid-rows.csv" 2> "$test_dir/error.log"

    diff -u - <(sort_account_rows "$test_dir/accounts-with-invalid-rows.csv") <<'CSV'
client,available,held,total,locked
42,0.0000,2.0000,2.0000,false
7,1.9999,0.0000,1.9999,true
CSV
    grep -Fq 'Could not read transaction:' "$test_dir/error.log"
    grep -Fq 'UnknownTransactionType("transfer")' "$test_dir/error.log"
}

test_rejects_invalid_headers_without_writing_accounts() {
    cat > "$test_dir/invalid-headers.csv" <<'CSV'
type,client,tx
deposit,7,1
CSV

    if cargo run --quiet --locked -- "$test_dir/invalid-headers.csv" \
        > "$test_dir/invalid-headers-output.csv" 2> "$test_dir/invalid-headers-error.log"; then
        echo 'Expected an invalid CSV header to fail' >&2
        return 1
    fi

    test ! -s "$test_dir/invalid-headers-output.csv"
    grep -Fq 'missing required CSV header "amount"' "$test_dir/invalid-headers-error.log"
}

test_processes_transactions_and_locks_charged_back_accounts
test_skips_invalid_rows_and_continues_processing
test_rejects_invalid_headers_without_writing_accounts

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

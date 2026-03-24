# pds_generator

A fast, allocation-free CLI tool written in Rust that generates sequential PDS
codes starting after a given PDS code.

## What is a PDS?

A PDS is a 5-character alphanumeric code matching the pattern `W[0-9A-Z]{4}`.\
Ordering is lexicographic over the base-36 alphabet `[0-9A-Z]`, so for example:

```
W1UL8 → W1UL9 → W1ULA → W1ULB → ... → W1ULZ → W1UM0
```

## Usage

```bash
pds_generator <START_PDS> <QUANTITY> [FORMAT]
```

### Arguments

| Argument    | Required | Description                                  |
| ----------- | -------- | -------------------------------------------- |
| `START_PDS` | ✅       | Starting PDS code — **excluded** from output |
| `QUANTITY`  | ✅       | Number of PDS codes to generate              |
| `FORMAT`    | ❌       | Output format: `CSV` (default) or `SQL`      |

### Examples

```bash
# CSV output (default) — redirect to file
./pds_generator W18A1 12999 > batch_march.csv

# CSV explicit
./pds_generator W18A1 12999 CSV > batch_march.csv

# SQL array — use for existence checks on the database
./pds_generator W18A1 100 SQL
```

## Output formats

### CSV (default)

Ready to import into DBeaver or any SQL client against the `rif_pds` table:

```
W18A2,N,,
W18A3,N,,
W18A4,N,,
...
```

### SQL

Array literal compatible with PostgreSQL `ANY(ARRAY[...])` queries:

```sql
['W18A2','W18A3','W18A4',...]
```

Useful for verifying that generated PDS codes do not already exist in the main
table:

```sql
SELECT pds_code
FROM contract_ps_data.pds
WHERE pds_code = ANY(ARRAY[
    'W18A2','W18A3',...
]);
```

## Build

```bash
cargo build --release
# binary will be at target/release/pds_generator
```

## Test

```bash
cargo test
```

## PDS bounds

| Bound   | Value   |
| ------- | ------- |
| Minimum | `W0000` |
| Maximum | `WZZZZ` |

Attempting to increment beyond `WZZZZ` returns an error.

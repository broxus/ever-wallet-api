CREATE INDEX transactions_lt_idx ON transactions (transaction_lt) WHERE transaction_lt IS NOT NULL;

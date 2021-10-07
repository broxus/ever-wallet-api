ALTER TABLE transactions ADD COLUMN transaction_timestamp TIMESTAMP;
CREATE INDEX transactions_timestamp_idx ON transactions (transaction_timestamp) WHERE transaction_timestamp IS NOT NULL;

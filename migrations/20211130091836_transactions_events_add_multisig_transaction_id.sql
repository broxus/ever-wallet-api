ALTER TABLE transactions ADD COLUMN multisig_transaction_id BIGINT;
ALTER TABLE transaction_events ADD COLUMN multisig_transaction_id BIGINT;
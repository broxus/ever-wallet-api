ALTER TABLE token_transactions ADD COLUMN sender_workchain_id INT;
ALTER TABLE token_transactions ADD COLUMN sender_hex VARCHAR(64);

ALTER TABLE token_transaction_events ADD COLUMN sender_workchain_id INT;
ALTER TABLE token_transaction_events ADD COLUMN sender_hex VARCHAR(64);

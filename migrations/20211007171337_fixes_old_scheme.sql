alter table api_service_key
    alter column whitelist type jsonb
        using to_jsonb(whitelist);

alter table address
    alter column custodians_public_keys type jsonb
        using to_jsonb(custodians_public_keys);

ALTER TABLE transactions ADD COLUMN messages_hash JSONB;
CREATE INDEX transactions_messages_hash_gin_idx ON transactions USING gin (messages_hash jsonb_path_ops);

ALTER TABLE transaction_events ADD COLUMN sender_workchain_id INT;
ALTER TABLE transaction_events ADD COLUMN sender_hex VARCHAR(64);

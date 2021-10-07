CREATE UNIQUE INDEX transactions_t_hash_a_wi_hex_d_idx ON transactions (transaction_hash, account_workchain_id, account_hex, direction)
    WHERE transaction_hash IS NOT NULL;

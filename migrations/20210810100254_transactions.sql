
DROP TYPE IF EXISTS twa_transaction_direction;

CREATE TYPE twa_transaction_direction as ENUM (
    'Send',
    'Receive'
    );

DROP TYPE IF EXISTS twa_transaction_status;

CREATE TYPE twa_transaction_status as ENUM (
    'New',
    'Done',
    'PartiallyDone',
    'Error'
    );

CREATE TABLE transactions (
                              id                          UUID NOT NULL,
                              service_id                  UUID NOT NULL,
                              message_hash                VARCHAR(64) NOT NULL,
                              transaction_hash            VARCHAR(64),
                              transaction_lt              NUMERIC,
                              transaction_timeout         BIGINT,
                              transaction_scan_lt         BIGINT,
                              sender_workchain_id         INT,
                              sender_hex                  VARCHAR(64),
                              account_workchain_id        INT NOT NULL,
                              account_hex                 VARCHAR(64) NOT NULL,
                              messages                    jsonb,
                              messages_hash               jsonb,
                              data                        jsonb,
                              original_value              NUMERIC,
                              original_outputs            jsonb,
                              value                       NUMERIC,
                              fee                         NUMERIC,
                              balance_change              NUMERIC,
                              direction                   twa_transaction_direction NOT NULL,
                              status                      twa_transaction_status NOT NULL,
                              error                       TEXT,
                              aborted                     BOOL NOT NULL,
                              bounce                      BOOL NOT NULL,
                              created_at                  TIMESTAMP NOT NULL DEFAULT current_timestamp,
                              updated_at                  TIMESTAMP NOT NULL DEFAULT current_timestamp,
                              CONSTRAINT transactions_pk PRIMARY KEY (id),
                              CONSTRAINT transactions_to_api_service_fk FOREIGN KEY (service_id) REFERENCES api_service (id),
                              CONSTRAINT transactions_account_wc_hex_to_address_fk FOREIGN KEY (account_workchain_id, account_hex) REFERENCES address(workchain_id, hex)
);

CREATE UNIQUE INDEX transactions_m_hash_account_wc_hex_idx ON transactions (message_hash, account_workchain_id, account_hex)
    WHERE direction = 'Send' AND transaction_hash IS NULL;
CREATE INDEX transactions_service_id_idx ON transactions (service_id);
CREATE INDEX transactions_m_hash_idx ON transactions (message_hash);
CREATE INDEX transactions_t_hash_idx ON transactions (transaction_hash);
CREATE INDEX transactions_account_wc_hex_idx ON transactions (account_workchain_id, account_hex);
CREATE INDEX transactions_created_at_idx ON transactions (created_at);
CREATE UNIQUE INDEX transactions_t_hash_a_wi_hex_d_idx ON transactions (transaction_hash, account_workchain_id, account_hex, direction)
    WHERE transaction_hash IS NOT NULL;
CREATE INDEX transactions_lt_idx ON transactions (transaction_lt) WHERE transaction_lt IS NOT NULL;
CREATE INDEX transactions_messages_hash_gin_idx ON transactions USING gin (messages_hash jsonb_path_ops);

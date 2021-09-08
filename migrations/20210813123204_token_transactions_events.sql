CREATE TABLE token_transaction_events (
        id                          UUID NOT NULL,
        service_id                  UUID NOT NULL,
        token_transaction_id        UUID NOT NULL,
        message_hash                VARCHAR(64) NOT NULL,
        account_workchain_id        INT NOT NULL,
        account_hex                 VARCHAR(64) NOT NULL,
        value                       NUMERIC NOT NULL,
        root_address                VARCHAR NOT NULL,
        transaction_direction       twa_transaction_direction NOT NULL,
        transaction_status          twa_token_transaction_status NOT NULL,
        event_status                twa_transaction_event_status NOT NULL,
        created_at                  TIMESTAMP NOT NULL DEFAULT current_timestamp,
        updated_at                  TIMESTAMP NOT NULL DEFAULT current_timestamp,
        CONSTRAINT token_transaction_events_pk PRIMARY KEY (id),
        CONSTRAINT token_transaction_events_to_api_service_fk FOREIGN KEY (service_id) REFERENCES api_service (id),
        CONSTRAINT token_transaction_events_to_transaction FOREIGN KEY (token_transaction_id) REFERENCES token_transactions(id),
        CONSTRAINT token_transaction_events_wc_hex_to_address_fk FOREIGN KEY (account_workchain_id, account_hex) REFERENCES address(workchain_id, hex)
);

CREATE UNIQUE INDEX token_transaction_events_transaction_id_status ON token_transaction_events (token_transaction_id, transaction_status);
CREATE INDEX token_transaction_events_service_id_idx ON token_transaction_events (service_id);
CREATE INDEX token_transaction_events_m_hash_idx ON token_transaction_events (message_hash);
CREATE INDEX token_transaction_events_account_wc_hex_idx ON token_transaction_events (account_workchain_id, account_hex);
CREATE INDEX token_transaction_events_created_at_idx ON token_transaction_events (created_at);
CREATE INDEX token_transaction_events_status_idx ON token_transaction_events (event_status);



DROP TYPE IF EXISTS twa_transaction_event_status;

CREATE TYPE twa_transaction_event_status as ENUM (
    'New',
    'Notified',
    'Error'
    );

CREATE TABLE transaction_events (
                                    id                          UUID NOT NULL,
                                    service_id                  UUID NOT NULL,
                                    transaction_id              UUID NOT NULL,
                                    message_hash                VARCHAR(64) NOT NULL,
                                    account_workchain_id        INT NOT NULL,
                                    account_hex                 VARCHAR(64) NOT NULL,
                                    sender_workchain_id         INT,
                                    sender_hex                  VARCHAR(64),
                                    balance_change              NUMERIC,
                                    transaction_direction       twa_transaction_direction NOT NULL,
                                    transaction_status          twa_transaction_status NOT NULL,
                                    event_status                twa_transaction_event_status NOT NULL,
                                    created_at                  TIMESTAMP NOT NULL DEFAULT current_timestamp,
                                    updated_at                  TIMESTAMP NOT NULL DEFAULT current_timestamp,
                                    CONSTRAINT transaction_events_pk PRIMARY KEY (id),
                                    CONSTRAINT transaction_events_to_api_service_fk FOREIGN KEY (service_id) REFERENCES api_service (id),
                                    CONSTRAINT transaction_events_to_transaction FOREIGN KEY (transaction_id) REFERENCES transactions(id),
                                    CONSTRAINT transaction_events_wc_hex_to_address_fk FOREIGN KEY (account_workchain_id, account_hex) REFERENCES address(workchain_id, hex)
);

CREATE UNIQUE INDEX transaction_events_transaction_id_status ON transaction_events (transaction_id, transaction_status);
CREATE INDEX transaction_events_service_id_idx ON transaction_events (service_id);
CREATE INDEX transaction_events_m_hash_idx ON transaction_events (message_hash);
CREATE INDEX transaction_events_account_wc_hex_idx ON transaction_events (account_workchain_id, account_hex);
CREATE INDEX transaction_events_created_at_idx ON transaction_events (created_at);
CREATE INDEX transaction_events_status_idx ON transaction_events (event_status);

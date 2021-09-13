DROP TYPE IF EXISTS twa_token_transaction_status;

CREATE TYPE twa_token_transaction_status as ENUM (
    'New',
    'Done',
    'Error'
    );

CREATE TABLE token_transactions
(
    id                   UUID NOT NULL,
    service_id           UUID NOT NULL,
    transaction_hash     VARCHAR(64),
    message_hash         VARCHAR(64) NOT NULL,
    owner_message_hash   VARCHAR(64),
    account_workchain_id INT NOT NULL,
    account_hex          VARCHAR(64) NOT NULL,
    value                DECIMAL NOT NULL,
    root_address         VARCHAR NOT NULL,
    payload              BYTEA,
    error                VARCHAR,
    block_hash           VARCHAR(64),
    block_time           INTEGER,
    direction            twa_transaction_direction NOT NULL,
    status               twa_token_transaction_status NOT NULL,
    created_at           TIMESTAMP NOT NULL DEFAULT current_timestamp,
    updated_at           TIMESTAMP NOT NULL DEFAULT current_timestamp,
    PRIMARY KEY (id),
    CONSTRAINT token_transactions_to_api_service_fk FOREIGN KEY (service_id) REFERENCES api_service (id),
    CONSTRAINT token_transactions_account_wc_hex_to_address_fk FOREIGN KEY (account_workchain_id, account_hex) REFERENCES address(workchain_id, hex)
);

CREATE INDEX token_transactions_service_id_idx ON token_transactions (service_id);
CREATE INDEX token_transactions_m_hash_idx ON token_transactions (message_hash);
CREATE INDEX token_transactions_owner_m_hash_idx ON token_transactions (owner_message_hash);
CREATE INDEX token_transactions_t_hash_idx ON token_transactions (transaction_hash);
CREATE INDEX token_transactions_created_at_idx ON token_transactions (created_at);
CREATE UNIQUE INDEX token_transactions_t_hash_a_wi_hex_d_idx ON token_transactions (transaction_hash, account_workchain_id, account_hex, direction)
    WHERE transaction_hash IS NOT NULL;

create or replace function update_token_balances_on_insert_in_token_transactions() returns trigger
    language plpgsql
as
$$
BEGIN
    INSERT INTO token_balances (service_id,
                                account_workchain_id,
                                account_hex,
                                balance,
                                root_address,
                                created_at,
                                updated_at
    )
    VALUES (NEW.service_id,
            NEW.account_workchain_id,
            NEW.account_hex,
            NEW.value,
            NEW.root_address,
            NEW.created_at,
            NEW.updated_at)
    ON CONFLICT (account_workchain_id, account_hex, root_address) DO UPDATE
        SET balance = token_balances.balance + CASE WHEN NEW.status = 'Done' THEN NEW.value ELSE 0 END , updated_at = NEW.updated_at
    WHERE token_balances.account_workchain_id = NEW.account_workchain_id
      AND token_balances.account_hex = NEW.account_hex
      AND token_balances.root_address = NEW.root_address;
    RETURN NEW;
END;
$$;

create trigger update_token_balances_trg
    after insert
    on token_transactions
    for each row
execute procedure update_token_balances_on_insert_in_token_transactions();

CREATE OR REPLACE FUNCTION update_token_balance_on_update() RETURNS TRIGGER AS
$$
BEGIN
    LOCK TABLE token_balances IN SHARE ROW EXCLUSIVE MODE;
    UPDATE token_balances
    SET (balance, updated_at) = (balance + CASE WHEN NEW.status = 'Done' THEN NEW.value ELSE 0 END - CASE WHEN OLD.status = 'Done' THEN OLD.value ELSE 0 END, current_timestamp)
    WHERE account_hex = NEW.account_hex AND account_workchain_id = NEW.account_workchain_id AND root_address = NEW.root_address;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER token_transactions_tg_update_update_token_balance
    AFTER UPDATE
    ON token_transactions
    FOR EACH ROW
EXECUTE PROCEDURE update_token_balance_on_update();

CREATE OR REPLACE FUNCTION update_token_balance_on_delete() RETURNS TRIGGER AS
$$
BEGIN
    LOCK TABLE token_balances IN SHARE ROW EXCLUSIVE MODE;
    UPDATE token_balances
    SET (balance, updated_at) = (balance -  CASE WHEN OLD.status = 'Done' THEN OLD.value ELSE 0 END, current_timestamp)
    WHERE account_hex = OLD.account_hex AND account_workchain_id = OLD.account_workchain_id AND root_address = OLD.root_address;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER token_transactions_tg_delete_update_token_balance
    AFTER DELETE
    ON token_transactions
    FOR EACH ROW
EXECUTE PROCEDURE update_token_balance_on_delete();
CREATE TABLE token_balances
(
    service_id           UUID NOT NULL,
    account_workchain_id INT NOT NULL,
    account_hex          VARCHAR(64) NOT NULL,
    balance              DECIMAL NOT NULL,
    root_address         VARCHAR NOT NULL,
    created_at           TIMESTAMP NOT NULL DEFAULT current_timestamp,
    updated_at           TIMESTAMP NOT NULL DEFAULT current_timestamp,
    PRIMARY KEY (account_workchain_id, account_hex, root_address),
    CONSTRAINT token_balances_to_api_service_fk FOREIGN KEY (service_id) REFERENCES api_service (id),
    CONSTRAINT token_balances_account_wc_hex_to_address_fk FOREIGN KEY (account_workchain_id, account_hex) REFERENCES address(workchain_id, hex)
);

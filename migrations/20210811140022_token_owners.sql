CREATE TABLE token_owners
(
    address                    VARCHAR NOT NULL,
    owner_account_workchain_id INT NOT NULL,
    owner_account_hex          VARCHAR(64) NOT NULL,
    root_address               VARCHAR NOT NULL,
    created_at                  TIMESTAMP NOT NULL DEFAULT current_timestamp,
    PRIMARY KEY (address)
);
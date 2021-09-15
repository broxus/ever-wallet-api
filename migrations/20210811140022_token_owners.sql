CREATE TABLE token_owners
(
    address                    VARCHAR NOT NULL,
    owner_account_workchain_id INT NOT NULL,
    owner_account_hex          VARCHAR(64) NOT NULL,
    root_address               VARCHAR NOT NULL,
    code_hash                  BYTEA   NOT NULL,
    created_at                 TIMESTAMP NOT NULL DEFAULT current_timestamp,
    PRIMARY KEY (address),
    UNIQUE (owner_account_workchain_id, owner_account_hex, root_address)
);

CREATE INDEX token_owners_account_workchain_id_idx ON token_owners (owner_account_workchain_id);
CREATE INDEX token_owners_account_hex_idx ON token_owners (owner_account_hex);
CREATE INDEX token_owners_root_address_idx ON token_owners (root_address);

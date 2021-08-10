CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

DROP TYPE IF EXISTS twa_account_type;

CREATE TYPE twa_account_type as ENUM (
    'HighloadWallet',
    'Wallet',
    'SafeMultisig'
    );

CREATE TABLE api_service (
                             id                          UUID NOT NULL DEFAULT uuid_generate_v4(),
                             name                        VARCHAR(255) NOT NULL,
                             created_at                  TIMESTAMP NOT NULL DEFAULT current_timestamp,
                             CONSTRAINT api_service_pk PRIMARY KEY (id)
);

CREATE TABLE api_service_key (
                                 id                          UUID NOT NULL DEFAULT uuid_generate_v4(),
                                 service_id                  UUID NOT NULL,
                                 key                         VARCHAR(128) NOT NULL,
                                 secret                      VARCHAR(128) NOT NULL,
                                 whitelist                   JSONB,
                                 created_at                  TIMESTAMP NOT NULL DEFAULT current_timestamp,
                                 CONSTRAINT api_service_key_pk PRIMARY KEY (id),
                                 CONSTRAINT api_service_key_key_uk UNIQUE (key),
                                 CONSTRAINT api_service_key_to_api_service_fk FOREIGN KEY (service_id) REFERENCES api_service (id)
);

CREATE TABLE address (
                         id                          UUID NOT NULL DEFAULT uuid_generate_v4(),
                         service_id                  UUID NOT NULL,
                         workchain_id                INT NOT NULL,
                         hex                         VARCHAR(64) NOT NULL,
                         base64url                   VARCHAR(48) NOT NULL,
                         public_key                  VARCHAR(64) NOT NULL,
                         private_key                 VARCHAR(128) NOT NULL,
                         account_type                twa_account_type NOT NULL,
                         custodians                  INT,
                         confirmations               INT,
                         custodians_public_keys      JSONB,
                         balance                     NUMERIC NOT NULL DEFAULT 0,
                         created_at                  TIMESTAMP NOT NULL DEFAULT current_timestamp,
                         updated_at                  TIMESTAMP NOT NULL DEFAULT current_timestamp,
                         CONSTRAINT address_pk PRIMARY KEY (id),
                         CONSTRAINT address_to_api_service_fk FOREIGN KEY (service_id) REFERENCES api_service (id)
);

CREATE UNIQUE INDEX address_workchain_id_hex_idx ON address (workchain_id, hex);

CREATE TABLE api_service_callback (
                                      id                          UUID NOT NULL DEFAULT uuid_generate_v4(),
                                      service_id                  UUID NOT NULL,
                                      callback                    TEXT NOT NULL,
                                      created_at                  TIMESTAMP NOT NULL DEFAULT current_timestamp,
                                      CONSTRAINT api_service_callback_pk PRIMARY KEY (id),
                                      CONSTRAINT api_service_callback_to_api_service_fk FOREIGN KEY (service_id) REFERENCES api_service (id)
);

CREATE UNIQUE INDEX api_service_callback_service_id_idx ON api_service_callback (service_id);


DROP TYPE IF EXISTS twa_task_status;

CREATE TYPE twa_task_status as ENUM (
    'Pending',
    'InProgress',
    'Done',
    'Error'
    );

DROP TYPE IF EXISTS twa_task_kind;

CREATE TYPE twa_task_kind as ENUM (
    'Rescan',
    'DelayedTransfer'
    );

CREATE TABLE tasks (
                       id                          UUID NOT NULL,
                       service_id                  UUID NOT NULL,
                       account_workchain_id        INT NOT NULL,
                       account_hex                 VARCHAR(64) NOT NULL,
                       status                      twa_task_status NOT NULL,
                       kind                        twa_task_kind NOT NULL,
                       data                        jsonb NOT NULL,
                       error                       TEXT,
                       created_at                  TIMESTAMP NOT NULL,
                       updated_at                  TIMESTAMP NOT NULL DEFAULT current_timestamp,
                       CONSTRAINT tasks_pk PRIMARY KEY (id),
                       CONSTRAINT tasks_to_api_service_fk FOREIGN KEY (service_id) REFERENCES api_service (id),
                       CONSTRAINT tasks_wc_hex_to_address_fk FOREIGN KEY (account_workchain_id, account_hex) REFERENCES address(workchain_id, hex)
);

CREATE INDEX tasks_service_id_idx ON tasks (service_id);
CREATE INDEX tasks_account_wc_hex_idx ON tasks (account_workchain_id, account_hex);
CREATE INDEX tasks_created_at_idx ON tasks (created_at);
CREATE INDEX tasks_status_idx ON tasks (status);
CREATE INDEX tasks_kind_idx ON tasks (kind);

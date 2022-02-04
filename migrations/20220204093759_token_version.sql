DROP TYPE IF EXISTS twa_token_wallet_version;

CREATE TYPE twa_token_wallet_version as ENUM (
    'OldTip3v4',
    'Tip3'
    );

ALTER TABLE token_owners ADD COLUMN version twa_token_wallet_version NOT NULL DEFAULT 'OldTip3v4';

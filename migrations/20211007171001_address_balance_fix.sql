DROP INDEX IF EXISTS address_balance_idx;
CREATE INDEX address_balance_idx ON address (balance);

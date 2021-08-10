CREATE INDEX address_balance_idx ON address (balance);

CREATE OR REPLACE FUNCTION update_account_balance_on_insert() RETURNS TRIGGER AS
$$
BEGIN
    LOCK TABLE address IN SHARE ROW EXCLUSIVE MODE;
    UPDATE address
    SET (balance, updated_at) = (balance + coalesce(NEW.balance_change, 0), current_timestamp)
    WHERE hex = NEW.account_hex AND workchain_id = NEW.account_workchain_id;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE OR REPLACE FUNCTION update_account_balance_on_update() RETURNS TRIGGER AS
$$
BEGIN
    LOCK TABLE address IN SHARE ROW EXCLUSIVE MODE;
    UPDATE address
    SET (balance, updated_at) = (balance + coalesce(NEW.balance_change, 0) - coalesce(OLD.balance_change, 0), current_timestamp)
    WHERE hex = NEW.account_hex AND workchain_id = NEW.account_workchain_id;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE OR REPLACE FUNCTION update_account_balance_on_delete() RETURNS TRIGGER AS
$$
BEGIN
    LOCK TABLE address IN SHARE ROW EXCLUSIVE MODE;
    UPDATE address
    SET (balance, updated_at) = (balance - coalesce(OLD.balance_change, 0), current_timestamp)
    WHERE hex = OLD.account_hex AND workchain_id = OLD.account_workchain_id;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER transactions_tg_insert_update_balance
    AFTER INSERT
    ON transactions
    FOR EACH ROW
EXECUTE PROCEDURE update_account_balance_on_insert();

CREATE TRIGGER transactions_tg_update_update_balance
    AFTER UPDATE
    ON transactions
    FOR EACH ROW
EXECUTE PROCEDURE update_account_balance_on_update();

CREATE TRIGGER transactions_tg_delete_update_balance
    AFTER DELETE
    ON transactions
    FOR EACH ROW
EXECUTE PROCEDURE update_account_balance_on_delete();



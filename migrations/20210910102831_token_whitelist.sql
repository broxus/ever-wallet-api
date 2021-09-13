CREATE TABLE token_whitelist
(
    name    VARCHAR NOT NULL,
    address VARCHAR NOT NULL,
    PRIMARY KEY (name, address)
);

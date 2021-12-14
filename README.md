<p align="center">
    <h3 align="center">Ton Wallet API</h3>
    <p align="center">
        <a href="/LICENSE">
            <img alt="GitHub" src="https://img.shields.io/github/license/broxus/ton-wallet-api" />
        </a>
    </p>
</p>

### Overview
The wallet http api for telegram open network client.


### Runtime requirements
- CPU: 4 cores, 2 GHz
- RAM: 8 GB
- Storage: 100 GB fast SSD
- Network: 100 MBit/s
- Postgres: 11 or higher

### How to run

To simplify the build and create some semblance of standardization in this repository
there is a set of scripts for configuring the ton-wallet-api.

NOTE: scripts are prepared and tested on **Ubuntu 20.04**. You may need to modify them a little for other distros.

1. ##### Setup ton-wallet-api service
   ```bash
   ./scripts/setup.sh -t native --database-url ${DATABASE_URL}
   ```

   DATABASE_URL - Postgres connection url (example: postgresql://postgres:postgres@127.0.0.1/ton_wallet_api)

   > At this stage, a systemd service `ton-wallet-api` is created. Configs and keys will be in `/etc/ton-wallet-api`
   > and TON node DB will be in `/var/db/ton-wallet-api`.

   **Do not start this service yet!**

2. ##### Prepare config
   Either add the environment variables to the `[Service]` section of unit file.
   It is located at `/etc/systemd/system/ton-wallet-api.service`.

   ```unit file (systemd)
   [Service]
   ...
   Environment=DB_HOST=db_host
   Environment=DB_USER=db_user
   Environment=DB_PASSWORD=db_password
   Environment=DB_NAME=ton_wallet_api
   Environment=API_SECRET=secret
   Environment=SALT=salt
   ...
   ```

   Or simply replace the `${..}` parameters in the config. It is located at `/etc/ton-wallet-api/config.yaml`.

   > API_SECRET and SALT env vars needed to encrypt/decrypt all addresses private keys.
   > API_SECRET has nothing to do with api service. Sorry for confused name.

3. ##### Create api service
   ```bash
     ./scripts/api_service.sh -t native --database-url ${DATABASE_URL} --id ${SERVICE_ID} --name ${SERVICE_NAME} --key ${SERVICE_KEY} --secret ${SERVICE_SECRET}
   ```

   DATABASE_URL - Postgres connection url (example: postgresql://postgres:postgres@127.0.0.1/ton_wallet_api) \
   SERVICE_ID - Service id (UUID4) (example: 1fa337bd-2947-4809-9a7a-f04b4f9b738a) \
   SERVICE_NAME - Service name (example: Test) \
   SERVICE_KEY - Public key (example: apiKey) \
   SERVICE_SECRET - Secret key (example: apiSecret)

   **We recommend to use [password generator](https://passwordsgenerator.net) for creating SERVICE_KEY and SERVICE_SECRET!**

4. ##### Enable and start ton-wallet-api service
   ```bash
   systemctl enable ton-wallet-api
   systemctl start ton-wallet-api

   # Optionally check if it is running normally. It will take some time to start.
   # ton-wallet-api is fully operational when it prints `listening on ${your_listen_address}`
   journalctl -fu relay
   ```

   > ton-wallet-api has a built-in Prometheus metrics exporter which is configured in the `metrics_settings` section of the config.
   > By default, metrics are available at `http://127.0.0.1:10000/`
   >
   > <details><summary><b>Response example:</b></summary>
   > <p>
   >
   > ```
   > ton_service_create_address_total_requests 0
   > ton_service_send_transaction_total_requests 0
   > ton_service_recv_transaction_total_requests 0
   > ton_service_send_token_transaction_total_requests 0
   > ton_service_recv_token_transaction_total_requests 0
   > ton_subscriber_ready 1
   > ton_subscriber_current_utime 1639490380
   > ton_subscriber_time_diff 4
   > ton_subscriber_shard_client_time_diff 7
   > ton_subscriber_mc_block_seqno 13179326
   > ton_subscriber_shard_client_mc_block_seqno 13179326
   > ton_subscriber_pending_message_count 0
   > ```
   >
   > </p>
   > </details>

5. ##### Add more root tokens to whitelist
   ```bash
     ./scripts/root_token.sh -t native --database-url ${DATABASE_URL} --name ${TOKEN_NAME} --address ${TOKEN_ADDRESS}
   ```

   DATABASE_URL - Postgres connection url (example: postgresql://postgres:postgres@127.0.0.1/ton_wallet_api) \
   TOKEN_NAME - Token name (example: WTON) \
   TOKEN_ADDRESS - Token address (example: 0:0ee39330eddb680ce731cd6a443c71d9069db06d149a9bec9569d1eb8d04eb37)

### Callbacks
API can send callbacks to services using it. One can set callback url in `api_service_callback` table for any service.
`service_id` and `callback` columns must be set. After receiving or sending new transactions or token transactions 
API will call web hook with POST method on `callback` url. Body will contain `AccountTransactionEvent` from swagger. 


### Swagger
When server starts locally the swagger schema can be accessible by http://localhost:8080/ton/v3/swagger.yaml.


### HMAC Authentication
[pre-request-script.js](scripts/pre-request-script.js) is javascript for using with Postman's pre-request script feature. It generates HTTP request headers for HMAC authentication.
Copy the contents of [pre-request-script.js](scripts/pre-request-script.js) into the "Pre-request Script" tab in Postman to send request.


### Example config

> NOTE: The syntax `${VAR}` can also be used everywhere in config. It will be
> replaced by the value of the environment variable `VAR`.

```yaml
---
# Server address
server_addr: "0.0.0.0:8080"
# Database URL
database_url: "postgresql://${DB_USER}:${DB_PASSWORD}@${DB_HOST}/${DB_NAME}"
# Database Connection Pools
db_pool_size: 5
ton_core:
  # UDP port, used for ADNL node. Default: 30303
  adnl_port: 30303
  # Root directory for relay DB. Default: "./db"
  db_path: "/var/ton-wallet-api/db"
  # Path to ADNL keys.
  # NOTE: Will be generated if it was not there.
  # Default: "./adnl-keys.json"
  keys_path: "/var/ton-wallet-api/adnl-keys.json"
metrics_settings:
  # Listen address of metrics. Used by the client to gather prometheus metrics.
  # Default: "127.0.0.1:10000"
  listen_address: "127.0.0.1:10000"
  # URL path to the metrics. Default: "/"
  # Example: `curl http://127.0.0.1:10000/`
  metrics_path: "/"
  # Metrics update interval in seconds. Default: 10
  collection_interval_sec: 10
# log4rs settings.
# See https://docs.rs/log4rs/1.0.0/log4rs/ for more details
logger_settings:
  appenders:
    stdout:
      kind: console
      encoder:
        pattern: "{d(%Y-%m-%d %H:%M:%S %Z)(utc)} - {h({l})} {M} = {m} {n}"
  root:
    level: error
    appenders:
      - stdout
  loggers:
    ton_wallet_api:
      level: info
      appenders:
        - stdout
      additive: false
    warp:
      level: info
      appenders:
        - stdout
      additive: false
    ton_indexer:
      level: error
      appenders:
        - stdout
      additive: false
```
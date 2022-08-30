<p align="center">
   <h3 align="center">Everscale Wallet API</h3>
    <p align="center">
        <a href="/LICENSE">
            <img alt="GitHub" src="https://img.shields.io/github/license/broxus/octusbridge-relay" />
        </a>
    </p>
</p>

### Overview
This is a light node + api for sending and tracking payments. The app listens for addresses from the database and
indexes all transactions, putting information about them in the postsgres DB. All transactions with native EVERs are
tracked, and there is a whitelist of root token addresses to be tracked in the settings. There is a callbacks table
in the database, where you can specify the url of your backend to which callbacks will come for all transactions.

It takes about 20 minutes to synchronize the node.
Both the ton-wallet-api and callback requests use HMAC signatures in the headers.

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
   Environment=SECRET=secret
   Environment=SALT=salt
   ...
   ```

   > SECRET - any string to encrypt/decrypt all addresses private keys.

   > SALT - 16 bytes recommended in B64 for secret hashing.

   ###### How to gen SALT
   ```bash
   cargo build --release
   ./target/release/ton-wallet-api salt
   ```

3. ##### Create api service
   ```bash
     ./scripts/api_service.sh -t native --database-url ${DATABASE_URL} --id ${SERVICE_ID} --name ${SERVICE_NAME} --key ${SERVICE_KEY} --secret ${SERVICE_SECRET}
   ```

   DATABASE_URL - Postgres connection url (example: postgresql://postgres:postgres@127.0.0.1/ton_wallet_api) \
   SERVICE_ID - Service id (UUID4) (example: 1fa337bd-2947-4809-9a7a-f04b4f9b738a) \
   SERVICE_NAME - Service name (example: test) \
   SERVICE_KEY - Public key (example: apiKey) \
   SERVICE_SECRET - Secret key (example: apiSecret)

4. ##### Enable and start ton-wallet-api service
   ```bash
   systemctl enable ton-wallet-api
   systemctl start ton-wallet-api

   # Optionally check if it is running normally. It will take some time to start.
   # ton-wallet-api is fully operational when it prints `listening on ${your_listen_address}`
   journalctl -fu ton-wallet-api
   ```

   > Wallet API has a two built-in Prometheus metrics exporters: API and Node.
   > You can enable API metrics by giving the value of api_metrics_addr in config.
   > Node metrics exporter is configured in the `node_metrics_settings` section of the config.
   > By default, node metrics are available at `http://127.0.0.1:10000/`
   >
   > <details><summary><b>Node metrics response example:</b></summary>
   > <p>
   >
   > ```
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

5. ##### Update service
   ```bash
     ./scripts/update.sh -t native --database-url ${DATABASE_URL}
   ```

   DATABASE_URL - Postgres connection url (example: postgresql://postgres:postgres@127.0.0.1/ton_wallet_api)


### Let's start using Wallet API

1. #### Create address
   Create yourself a "system address" by calling `/address/create` with empty parameters. The response will return a EVER
   address. It is necessary to send EVERs on it, which will be consumed as gas for further work.
   
   **For simplicity, you use the script**

   ```bash
   API_KEY=${API_KEY} SECRET=${API_SECRET} HOST=${HOST} \
   ./scripts/wallet.sh -m create_account
   ```

2. #### Callbacks
   In the table `api_service_callback` we enter the address of our backend, which will deal with payment processing.
   After receiving or sending new transactions or token transactions Wallet API will call web hook with POST method on
   `callback` url. Body will contain `AccountTransactionEvent` from [swagger](https://tonapi.broxus.com/swagger.yaml).

3. #### Token Whitelist
   You can see the root-contract addresses at [manifest](https://raw.githubusercontent.com/broxus/ton-assets/master/manifest.json).
   By default, the whitelist already includes all the tokens in this list.

   To add more tokens to the whitelist, use the script:
   ```bash
     ./scripts/root_token.sh -t native --database-url ${DATABASE_URL} --name ${TOKEN_NAME} --address ${TOKEN_ADDRESS}
   ```
   
   DATABASE_URL - Postgres connection url (example: postgresql://postgres:postgres@127.0.0.1/ton_wallet_api) \
   TOKEN_NAME - Token name (example: WTON) \
   TOKEN_ADDRESS - Token address (example: 0:0ee39330eddb680ce731cd6a443c71d9069db06d149a9bec9569d1eb8d04eb37)

4. #### Transfer EVER
   Example request:
   ```
   /transactions/create
   {
      // a random uuid that you generate yourself and store on your backend, to further track the status of the transaction
      "id":"00000000-0000-0000-0000-000000000000",
      // The address of the sender. For example, your system address.
      "fromAddress":"0:0000000000000000000000000000000000000000000000000000000000000000",
      "bounce":false,
      "outputs":[
         {
            // how much EVER to send. To send 1 EVER this value = 1000000000
            "value":"1000000000",
            // Set Normal to take the number of sent EVERs from the value
            "outputType":"Normal",
            // Recipient address of EVERs
            "recipientAddress":"0:0000000000000000000000000000000000000000000000000000000000000000"
         }
      ]
   }
   ```
   Or use the script:
   ```bash
   # Create transaction
   API_KEY=${API_KEY} SECRET=${API_SECRET} HOST=${HOST} \
   ./scripts/wallet.sh -m create_transaction \
   --src-addr {sender} --dst-addr {recipient} --amount {amount}
   ```
   
   You can track the status of a transaction with:
   1) (Recommended way) via callback `AccountTransactionEvent`, which has transactionStatus field:
      * `expired` - end state for failed transactions,
      * `done` - final state for successful transactions. 
      
      If your backend was disabled during the callback or responded with an error, the event will have an `Error` state.
      In this case you should query all events `/events` in `Error` state at backend startup, process them and give each
      event a `Done` state by calling `/events/mark`.
   2) by polling the GET method `/transactions/id/<uuid>`

5. #### How to process a payment from a user on the backend
   We generate a deposit address for the user by calling `/address/create` with empty parameters. After receiving the
   payment, the backend receives a callback of the form `AccountTransactionEvent` (see [swagger](https://tonapi.broxus.com/swagger.yaml)).
   You can also get such events in a list, using the /events method.

   If your backend was not working at the time of the callback or responded with an error, the event will have an
   `Error` status. In this case you should query all events `/events` in `Error` state at the start of the backend, process
   them and set each of them to `Done` state by calling `/events/mark`. Each event has an id (generated by ton-api). You
   can do extra checks on it to make sure that your backend doesn't re-process events.

6. #### Transfer tokens
   First, check the status and balance of the address you want to send tokens from by making a GET request to /address/{string}.
   The address you are sending tokens from must have at least 0.6 EVER (balance >= 600000000).

   To transfer tokens, use the method:
   ```
   /tokens/transactions/create
   {
      // a random uuid that you generate yourself and store on your backend, to further track the status of the transaction
      "id":"00000000-0000-0000-0000-000000000000",
      // The address of the sender. For example, your system address.
      "fromAddress":"0:0000000000000000000000000000000000000000000000000000000000000000",
      // Recipient address of EVERs
      "recipientAddress":"0:0000000000000000000000000000000000000000000000000000000000000000",
      // The number of tokens with decimals. For example, for transferring 1 USDT this value = "1000000"
      "value":"1000000000",
      // How much to apply EVER, the default recommended value is 0.5 EVER. The funds will be debited fromAddress.
      "fee": "5000000000",
      // The address to which to return the residuals EVER. For example, your system address.
      "sendGasTo":"0:0000000000000000000000000000000000000000000000000000000000000000",
      // Token Address from whitelist
      "rootAddress":"0:0000000000000000000000000000000000000000000000000000000000000000"
   }
   ```
   Or use the script:
   ```bash
    # Create token transaction
   API_KEY=${API_KEY} SECRET=${API_SECRET} HOST=${HOST} \
   ./scripts/wallet.sh -m create_token_transaction \
   --src-addr {sender} --dst-addr {recipient} \
   --root-addr {root_token_address} --amount {amount}
   ```

   You can track the status of a transaction with:
   1) (Recommended way) via callback `AccountTransactionEvent`, which has `transactionStatus` field:
      * `expired` - end state for failed transactions,
      * `done` - final state for successful transactions.

      If your backend was down at the time of the callback or responded with an error, the event will have an `Error` state.
      In this case, you should query all events `/token/events` in Error state at backend startup, process them, and set
      each event to `Done` state by calling `/token/events/mark`.

   2) by polling with the GET method `/tokens/transactions/id/<uuid>`


### Postman
[pre-request-script.js](scripts/pre-request-script.js) is javascript for using with Postman's pre-request script feature. It generates HTTP request headers for HMAC authentication.
Copy the contents of [pre-request-script.js](scripts/pre-request-script.js) into the "Pre-request Script" tab in Postman to send signed request.

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
  # Root directory for ton-wallet-api DB. Default: "./db"
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
    ton_indexer:
      level: error
      appenders:
        - stdout
      additive: false
```

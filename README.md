<p align="center">
   <h3 align="center">Tycho Wallet API</h3>
    <p align="center">
        <a href="/LICENSE">
            <img alt="GitHub" src="https://img.shields.io/github/license/broxus/octusbridge-relay" />
        </a>
    </p>
</p>

### Overview
This is a light node + api for sending and tracking payments. The app listens for addresses from the database and
indexes all transactions, putting information about them in the postsgres DB. All transactions with native TYCHOs are
tracked, and there is a whitelist of root token addresses to be tracked in the settings. There is a callbacks table
in the database, where you can specify the url of your backend to which callbacks will come for all transactions.

It takes about 20 minutes to synchronize the node.
Both the tycho-wallet-api and callback requests use HMAC signatures in the headers.

### Runtime requirements
- CPU: 4 cores, 2 GHz
- RAM: 8 GB
- Storage: 100 GB fast SSD
- Network: 100 MBit/s
- Postgres: 11 or higher

### How to run natively

To simplify the build and create some semblance of standardization in this repository
there is a set of scripts for configuring the tycho-wallet-api.

NOTE: scripts are prepared and tested on **Ubuntu 20.04**. You may need to modify them a little for other distros.

1. ##### Setup tycho-wallet-api service
   ```bash
   ./scripts/setup.sh -t native --database-url ${DATABASE_URL}
   ```

   DATABASE_URL - Postgres connection url (example: postgresql://postgres:postgres@127.0.0.1/tycho_wallet_api)

   > At this stage, a systemd service `tycho-wallet-api` is created. Configs and keys will be in `/etc/tycho-wallet-api`
   > and TON node DB will be in `/var/db/tycho-wallet-api`.

   **Do not start this service yet!**

2. ##### Prepare config
   Either add the environment variables to the `[Service]` section of unit file.
   It is located at `/etc/systemd/system/tycho-wallet-api.service`.

   ```unit file (systemd)
   [Service]
   ...
   Environment=DB_HOST=db_host
   Environment=DB_USER=db_user
   Environment=DB_PASSWORD=db_password
   Environment=DB_NAME=tycho_wallet_api
   Environment=SECRET=secret
   Environment=SALT=salt
   ...
   ```

   > SECRET - any string to encrypt/decrypt all addresses private keys.

   > SALT - 16 bytes recommended in B64 for secret hashing.

   ###### How to gen SALT
   ```bash
   cargo build --release
   ./target/release/tycho-wallet-api salt
   ```
   
   ###### How to gen config
   ```bash
   cargo build --release
   SALT=salt SECRET=secret ./target/release/tycho-wallet-api server --init-config config.json
   ```

3. ##### Create api service
   ```bash
     ./scripts/api_service.sh -t native --database-url ${DATABASE_URL} --id ${SERVICE_ID} --name ${SERVICE_NAME} --key ${SERVICE_KEY} --secret ${SERVICE_SECRET}
   ```

   DATABASE_URL - Postgres connection url (example: postgresql://postgres:postgres@127.0.0.1/tycho_wallet_api) \
   SERVICE_ID - Service id (UUID4) (example: 1fa337bd-2947-4809-9a7a-f04b4f9b738a) \
   SERVICE_NAME - Service name (example: test) \
   SERVICE_KEY - Public key (example: apiKey) \
   SERVICE_SECRET - Secret key (example: apiSecret)

4. ##### Enable and start tycho-wallet-api service
   ```bash
   systemctl enable tycho-wallet-api
   systemctl start tycho-wallet-api

   # Optionally check if it is running normally. It will take some time to start.
   # tycho-wallet-api is fully operational when it prints `listening on ${your_listen_address}`
   journalctl -fu tycho-wallet-api
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

   DATABASE_URL - Postgres connection url (example: postgresql://postgres:postgres@127.0.0.1/tycho_wallet_api)


### Let's start using Wallet API

1. #### Create address
   Create yourself a "system address" by calling `/address/create` with empty parameters. The response will return a TYCHO
   address. It is necessary to send TYCHOs on it, which will be consumed as gas for further work. Default used account type is
   `Wallet v5 r1`.
   
   **For simplicity, you use the script**

   ```bash
   API_KEY=${API_KEY} SECRET=${API_SECRET} HOST=${HOST} \
   ./scripts/wallet.sh -m create_account --account-type WalletV5R1
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
     ./scripts/root_token.sh -t native --database-url ${DATABASE_URL} --name ${TOKEN_NAME} --address ${TOKEN_ADDRESS} --version ${TOKEN_CONTRACT_VERSION}
   ```
   
   DATABASE_URL - Postgres connection url (example: postgresql://postgres:postgres@127.0.0.1/tycho_wallet_api) \
   TOKEN_NAME - Token name (example: WTON) \
   TOKEN_ADDRESS - Token address (example: 0:0ee39330eddb680ce731cd6a443c71d9069db06d149a9bec9569d1eb8d04eb37)
   TOKEN_CONTRACT_VERSION - "Tip3" or "OldTip3v4"

4. #### Transfer TYCHO
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
            // how much TYCHO to send. To send 1 TYCHO this value = 1000000000
            "value":"1000000000",
            // Set Normal to take the number of sent TYCHOs from the value
            "outputType":"Normal",
            // Recipient address of TYCHOs
            "recipientAddress":"0:0000000000000000000000000000000000000000000000000000000000000000"
         }
      ],
      // base64 encoded payload
      payload: "te6ccgEBAQEABgAACAVriBQ="
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

   To confirm a pending multisig transaction, use the script:
   ```bash
   API_KEY=${API_KEY} SECRET=${API_SECRET} HOST=${HOST} \
   ./scripts/wallet.sh -m confirm_transaction \
   --address {multisig_wallet_address} --transaction-id {transaction_id}
   ```

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
   The address you are sending tokens from must have at least 0.6 TYCHO (balance >= 600000000).

   To transfer tokens, use the method:
   ```
   /tokens/transactions/create
   {
      // a random uuid that you generate yourself and store on your backend, to further track the status of the transaction
      "id":"00000000-0000-0000-0000-000000000000",
      // The address of the sender. For example, your system address.
      "fromAddress":"0:0000000000000000000000000000000000000000000000000000000000000000",
      // Recipient address of TYCHOs
      "recipientAddress":"0:0000000000000000000000000000000000000000000000000000000000000000",
      // The number of tokens with decimals. For example, for transferring 1 USDT this value = "1000000"
      "value":"1000000000",
      // How much to apply TYCHO, the default recommended value is 0.5 TYCHO. The funds will be debited fromAddress.
      "fee": "5000000000",
      // The address to which to return the residuals TYCHO. For example, your system address.
      "sendGasTo":"0:0000000000000000000000000000000000000000000000000000000000000000",
      // Token Address from whitelist
      "rootAddress":"0:0000000000000000000000000000000000000000000000000000000000000000",
      // base64 encoded payload
      "payload": "te6ccgEBAQEABgAACAVriBQ="
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
[pre-request-script.js](scripts/pre-request-script.js) is javascript for using with Postman's pre-request script 
feature. It generates HTTP request headers for HMAC authentication. Copy the contents of 
[pre-request-script.js](scripts/pre-request-script.js) into the "Pre-request Script" tab in Postman to send signed 
request.

### Authorization signatures

A request must undergo an authorization step, which involves verifying the request payload, URI, and timestamp 
against a known secret specific to each service. For GET requests, the body is empty, while for POST requests, 
the body is a string representing the serialized JSON payload. The payload must be pretty-printed (stored) with an 
indentation of 4 spaces. An example of the string to be signed is provided below. This setup generally applies to POST 
requests.

```bash
stringToSign="$timestamp$uri$body"
echo -en "$stringToSign" | openssl sha256 -hmac "$secret" -binary | base64
```

### Example config

> NOTE: The syntax `${VAR}` can also be used everywhere in config. It will be
> replaced by the value of the environment variable `VAR`.

```json
{
  "public_ip": null,
  "local_ip": "0.0.0.0",
  "port": 30000,
  "network": {
    "quic": null,
    "connection_manager_channel_capacity": 128,
    "connectivity_check_interval": "5s",
    "max_frame_size": "8.4 MB",
    "connect_timeout": "10s",
    "connection_backoff": "10s",
    "max_connection_backoff": "1m",
    "connection_error_delay": "3s",
    "max_concurrent_outstanding_connections": 100,
    "max_concurrent_connections": null,
    "active_peers_event_channel_capacity": 128,
    "max_concurrent_requests_per_peer": 128,
    "shutdown_idle_timeout": "1m",
    "enable_0rtt": false,
    "connection_metrics": null
  },
  "dht": {
    "max_k": 6,
    "max_peer_info_ttl": "1h",
    "max_stored_value_ttl": "1h",
    "max_storage_capacity": "16.8 MB",
    "storage_item_time_to_idle": null,
    "local_info_refresh_period": "1m",
    "local_info_announce_period": "10m",
    "local_info_announce_period_max_jitter": "1m",
    "routing_table_refresh_period": "10m",
    "routing_table_refresh_period_max_jitter": "1m",
    "announced_peers_channel_capacity": 10
  },
  "peer_resolver": {
    "max_parallel_resolve_requests": 100,
    "min_ttl_sec": 600,
    "update_before_sec": 1200,
    "fast_retry_count": 10,
    "min_successfull_resolve_interval": "1m",
    "min_retry_interval": "1s",
    "max_retry_interval": "2m",
    "stale_retry_interval": "10m"
  },
  "overlay": {
    "public_overlay_peer_store_period": "3m",
    "public_overlay_peer_store_max_jitter": "30s",
    "public_overlay_peer_store_max_entries": 20,
    "public_overlay_peer_exchange_period": "3m",
    "public_overlay_peer_exchange_max_jitter": "30s",
    "public_overlay_peer_collect_period": "10s",
    "public_overlay_peer_collect_max_jitter": "5s",
    "public_overlay_peer_discovery_period": "3m",
    "public_overlay_peer_discovery_max_jitter": "30s",
    "exchange_public_entries_batch": 20
  },
  "public_overlay_client": {
    "neighbors": {
      "update_interval": "2m",
      "ping_interval": "30s",
      "apply_score_interval": "10s",
      "keep": 5,
      "max_ping_tasks": 5,
      "default_roundtrip": "300ms",
      "send_timeout": {
        "secs": 0,
        "nanos": 500000000
      },
      "query_timeout": {
        "secs": 1,
        "nanos": 0
      }
    },
    "validators": {
      "ping_interval": "1m",
      "ping_timeout": "1s",
      "keep": 5,
      "max_ping_tasks": 5,
      "send_timeout": {
        "secs": 0,
        "nanos": 500000000
      }
    }
  },
  "storage": {
    "root_dir": "./db",
    "rocksdb_enable_metrics": true,
    "rocksdb_lru_capacity": "134.2 MB",
    "cells_cache_size": "1.9 GB",
    "archive_chunk_size": "1024.0 KB",
    "split_block_tasks": 100,
    "archives_gc": {
      "persistent_state_offset": "5m"
    },
    "states_gc": {
      "random_offset": true,
      "interval": "1m"
    },
    "blocks_gc": {
      "type": "BeforeSafeDistance",
      "safe_distance": 1000,
      "min_interval": "1m",
      "enable_for_sync": true,
      "max_blocks_per_batch": 100000
    },
    "blocks_cache": {
      "ttl": "5m",
      "size": "500.0 MB"
    }
  },
  "blockchain_rpc_client": {
    "min_broadcast_timeout": "100ms",
    "too_new_archive_threshold": 4,
    "download_retries": 10
  },
  "blockchain_rpc_service": {
    "max_key_blocks_list_len": 8,
    "serve_persistent_states": true
  },
  "blockchain_block_provider": {
    "get_next_block_polling_interval": "1s",
    "get_block_polling_interval": "1s",
    "get_next_block_timeout": "2m",
    "get_block_timeout": "1m"
  },
  "archive_block_provider": {
    "max_archive_to_memory_size": "100.0 MB"
  },
  "rpc": null,
  "metrics": {
    "listen_addr": "127.0.0.1:10000"
  },
  "threads": {
    "rayon_threads": 8,
    "tokio_workers": 8
  },
  "profiling": {
    "profiling_dir": ""
  },
  "logger_config": {
    "outputs": [
      {
        "type": "Stderr"
      }
    ]
  },
  "starter": {
    "custom_boot_offset": null
  },
  "api": {
    "server_addr": "${SERVER_ADDR}",
    "database_url": "postgresql://postgres:postgres@${DATABASE_ADDR}:5432/tycho_wallet_api",
    "db_pool_size": 8,
    "key": [
      101,
      247,
      113,
      21,
      159,
      244,
      255,
      167,
      16,
      129,
      32,
      34,
      161,
      5,
      240,
      97,
      250,
      232,
      42,
      55,
      221,
      67,
      175,
      105,
      229,
      178,
      134,
      166,
      208,
      30,
      180,
      236
    ],
    "api_metrics_addr": null,
    "node_metrics_settings": null,
    "logger_settings": {
      "appenders": {
        "stdout": {
          "kind": "console",
          "encoder": {
            "pattern": "{d(%Y-%m-%d %H:%M:%S %Z)(utc)} - {h({l})} {M} = {m} {n}"
          }
        }
      },
      "root": {
        "level": "info",
        "appenders": [
          "stdout"
        ]
      },
      "loggers": {
        "tycho_wallet_api": {
          "level": "info",
          "appenders": [
            "stdout"
          ],
          "additive": false
        }
      }
    }
  }
}
```

### How to Run via Docker/Podman

This guide explains how to build and run the project using Podman (or Docker as an alternative). 
There are two Dockerfiles: one for building an intermediate image for setting and compiling the Rust project and 
another for deploying it. With that user may replace configuration files and start parameters and promptly rebuild
deployment image.

#### Prerequisites

User must provide a link to running Postgres instance with user and password. It will be used for migrations and for 
deployment as well.

#### Building Images

To build the images, two Dockerfiles are used: `builder.dockerfile` for compiling the project and `deploy.dockerfile` 
for setting up the deployment environment.

1. **Build the builder image**:
   The `builder.dockerfile` is responsible for compiling the project using Rust. It builds the project based on the 
   specified network (either `tycho` or other) and prepares the database for the application using SQLx.

   Use the following command to build the builder image:

   ```bash
   podman build --layers --network=host -f builder.dockerfile -t builder --build-arg DATABASE_URL="postgresql://tycho:tycho@localhost:5432/tycho"
   ```

2. **Build the deployment image**:
   The `deploy.dockerfile` is used to create the runtime environment and copy the necessary binaries and configuration 
   files from the `builder` stage.

   Build the deployment image using the following command:

   ```bash
   podman build --layers -f deploy.dockerfile -t tycho-wallet
   ```

#### Running the Container

Once the images are built, you can run the container using Podman or Docker.

1. **Running the application**:

   To run the application, use the following command:

   ```bash
   podman run --network=host tycho-wallet
   ```

   This will run the `tycho-wallet-api` server using the default configuration files already existing in the container.
   Errors shall be expected at this step.

   ```bash
   WARN: Environment variable DB_USER was not set
   WARN: Environment variable DB_PASSWORD was not set
   WARN: Environment variable DB_HOST was not set
   WARN: Environment variable DB_NAME was not set
   ```

   To avoid these errors, the database connection and other settings (such as secrets) shall be provided. 
   Alternatively, they may be added into `deploy.dockerfile`. For example:

   ```bash
   podman run --network=host \
     -v /tmp/tycho-data:/var/db/tycho-wallet-api
     -e DB_USER=tycho \
     -e DB_PASSWORD=tycho \
     -e DB_HOST=localhost \
     -e DB_NAME=tycho \
     -e SECRET=0xAAAAA \
     -e SALT=OreOYYe5nHWTHnOPSvsmMQ \
     tycho-wallet
   ```

   It generally allows dynamically setting environment variables for database credentials, secrets, and other 
   configurations.

### Troubleshooting

When the node is out of sync, removing database and re-syncing node may help to
restore service operations.

`rm -rf /var/db/tycho-wallet-api`

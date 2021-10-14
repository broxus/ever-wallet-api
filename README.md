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
- CPU: 8 cores, 2 GHz
- RAM: 16 GB
- Storage: 200 GB fast SSD
- Network: 100 MBit/s


### Native build

#### Requirements
- Rust 1.54+
- Clang 11

#### How to run
```bash
# Set 'salt' and 'secret' env vars
export SALT=${SALT}
export API_SECRET=${API_SECRET}

# Download network global config
wget https://raw.githubusercontent.com/tonlabs/main.ton.dev/master/configs/main.ton.dev/ton-global.config.json

# Run
RUSTFLAGS='-C target-cpu=native' cargo run \
  --release -- \
  --config config.yaml server --global-config ton-global.config.json
```

When node syncs and server starts you will see next messages:

```log
2021-09-23 16:19:19 UTC - INFO ton_wallet_api_lib::ton_core::ton_subscriber = TON subscriber is ready
2021-09-23 16:19:19 UTC - INFO warp::server = Server::run; addr=127.0.0.1:8080
2021-09-23 16:19:19 UTC - INFO warp::server = listening on http://127.0.0.1:8080
```


### Tips and tricks
Before running service you should create <b>api service</b> and <b>api service key</b>.

#### Create api service
```bash
DATABASE_URL=${DATABASE_URL} RUSTFLAGS='-C target-cpu=native' cargo run \
  --release -- api_service \
  --name ${SERVICE_NAME} --id ${SERVICE_ID}
```

#### Create api service key
```bash
DATABASE_URL=${DATABASE_URL} RUSTFLAGS='-C target-cpu=native' cargo run \
  --release -- api_service_key \
  --id ${SERVICE_ID} --key ${KEY} --secret ${SECRET}
```

#### Add root token to whitelist
```bash
# WTON as example
DATABASE_URL=${DATABASE_URL} RUSTFLAGS='-C target-cpu=native' cargo run \
  --release -- root_token \
  --name WTON --address 0:0ee39330eddb680ce731cd6a443c71d9069db06d149a9bec9569d1eb8d04eb37
```


### Swagger
When server starts locally the swagger schema can be accessible by http://localhost:8080/ton/v3/swagger.yaml.


### HMAC Authentication
[pre-request-script.js](pre-request-script.js) is javascript for using with Postman's pre-request script feature. It generates HTTP request headers for HMAC authentication.
Copy the contents of [pre-request-script.js](pre-request-script.js) into the "Pre-request Script" tab in Postman to send request.


### Example config

`config.yaml`

> NOTE: all parameters can be overwritten from environment

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

{ config, lib, pkgs, ... }:
with lib;  # use the functions from lib, such as mkIf
let
  # the values of the options set for the service by the user of the service
  cfg = config.services.ever-wallet-api;
in {
  ##### interface. here we define the options that users of our service can specify
  options = {
    # the options for our service will be located under services.ever-wallet-api
    services.ever-wallet-api = {
      enable = mkOption {
        type = types.bool;
        default = true;
        description = ''
          Whether to enable ever-wallet-api node by default.
        '';
      };
      package = mkOption {
        type = types.package;
        default = pkgs.ever-wallet-api;
        defaultText = "pkgs.ever-wallet-api";
        description = ''
          Which ever-wallet-api package to use with the service.
        '';
      };

      host = mkOption {
        type = types.str;
        default = "0.0.0.0";
        description = ''
          Which host address to bind to.
        '';
      };
      port = mkOption {
        type = types.int;
        default = 7354;
        description = ''
          Which port the service listens.
        '';
      };
      chain = mkOption {
        type = types.str;
        default = "Everscale";
        description = ''
          Which blockchain to use: Everscale, Venom
        '';
      };

      datadir = mkOption {
        type = types.str;
        default = "/var/lib/ever-wallet-api";
        description = ''
          Path to service state on filesystem.
        '';
      };
      configdir = mkOption {
        type = types.str;
        default = "ever-wallet-api";
        description = ''
          Path to service configs and keys on filesystem. The /etc/ prefix is appended automatically.
        '';
      };

      dbHost = mkOption {
        type = types.str;
        default = "127.0.0.1";
        description = ''
          Which address of PostgreSQL.
        '';
      };
      dbPort = mkOption {
        type = types.int;
        default = 5432;
        description = ''
          Which port the PostgreSQL listens.
        '';
      };
      dbUser = mkOption {
        type = types.str;
        default = "ever-wallet-api";
        description = ''
          Which username to use in PostgreSQL.
        '';
      };
      dbDatabase = mkOption {
        type = types.str;
        default = "ever-wallet-api";
        description = ''
          Which database name to use in PostgreSQL.
        '';
      };

      adnlPort = mkOption {
        type = types.int;
        default = 30303;
        description = ''
          UDP port, used for ADNL node. Default: 30303
        '';
      };

      metricsHost = mkOption {
        type = types.str;
        default = "127.0.0.1";
        description = ''
          Which address to bind metrics service to.
        '';
      };
      metricsPort = mkOption {
        type = types.int;
        default = 10000;
        description = ''
            Which port the metrics service listens.
        '';
      };
      metricsPath = mkOption {
        type = types.str;
        default = "/";
        description = ''
            URL path to the metrics. Default: "/"
        '';
      };
      metricsInterval = mkOption {
        type = types.int;
        default = 10;
        description = ''
            Metrics update interval in seconds. Default: 10
        '';
      };

      config = mkOption {
        type = types.str;
        description = ''
          Configuration file for server.
        '';
        default = ''
            # Server address
            server_addr: "${cfg.host}:${builtins.toString cfg.port}"
            # Database URL
            database_url: "postgresql://${cfg.dbUser}:''${DB_PASSWORD}@${cfg.dbHost}/${cfg.dbDatabase}"
            # Database Connection Pools
            db_pool_size: 5
            ton_core:
                # UDP port, used for ADNL node. Default: 30303
                adnl_port: ${builtins.toString cfg.adnlPort}
                # Root directory for ton-wallet-api DB. Default: "./db"
                db_path: "${cfg.datadir}/db"
                # Path to ADNL keys.
                # NOTE: Will be generated if it was not there.
                # Default: "./adnl-keys.json"
                keys_path: "${cfg.datadir}/adnl-keys.json"
                recover_indexer: true
            # Or metrics server won't start
            api_metrics_addr: "0.0.0.0:10000" 
            metrics_settings:
                # Listen address of metrics. Used by the client to gather prometheus metrics.
                # Default: "127.0.0.1:10000"
                listen_address: "${cfg.metricsHost}:${builtins.toString cfg.metricsPort}"
                # URL path to the metrics. Default: "/"
                # Example: `curl http://127.0.0.1:10000/`
                metrics_path: "${cfg.metricsPath}"
                # Metrics update interval in seconds. Default: 10
                collection_interval_sec: ${builtins.toString cfg.metricsInterval}
            # log4rs settings.
            # See https://docs.rs/log4rs/1.0.0/log4rs/ for more details
            logger_settings:
              appenders:
                stdout:
                  kind: console
                  encoder:
                    pattern: "{h({l})} {M} = {m} {n}"
              root:
                level: error
                appenders:
                  - stdout
              loggers:
                ton_wallet_api:
                  level: debug
                  appenders:
                    - stdout
                  additive: false
                tower_http:
                  level: debug
                  appenders:
                    - stdout
                  additive: false
                ton_indexer:
                  level: warn
                  appenders:
                    - stdout
                  additive: false
                tiny_adnl:
                  level: debug
                  appenders:
                    - stdout
                  additive: false

        '';
      };

      dbPasswordFile = mkOption {
        type = types.str;
        default = "/run/keys/everwalletapidb";
        description = ''
          Location of file with password for RPC.
        '';
      };
      dbPasswordFileService = mkOption {
        type = types.str;
        default = "everwalletapidb-key.service";
        description = ''
          Service that indicates that dbPasswordFile is ready.
        '';
      };
      everSecretFile = mkOption {
        type = types.str;
        default = "/run/keys/everwalletapisecret";
        description = ''
          Location of file with secret for decrypting transactions.
        '';
      };
      everSecretFileService = mkOption {
        type = types.str;
        default = "everwalletapisecret-key.service";
        description = ''
          Service that indicates that everSecretFile is ready.
        '';
      };
      everSaltFile = mkOption {
        type = types.str;
        default = "/run/keys/everwalletapisalt";
        description = ''
          Location of file with salt for ???.
        '';
      };
      everSaltFileService = mkOption {
        type = types.str;
        default = "everwalletapisalt-key.service";
        description = ''
          Service that indicates that everSaltFile is ready.
        '';
      };
    };
  };

  ##### implementation
  config = mkIf cfg.enable { # only apply the following settings if enabled
    # User to run the node
    users.users.ever-wallet-api = {
      name = "ever-wallet-api";
      group = "ever-wallet-api";
      extraGroups = [ ];
      description = "ever-wallet-api daemon user";
      home = cfg.datadir;
      isSystemUser = true;
    };
    users.groups.ever-wallet-api = {};
    environment.etc."${cfg.configdir}/config.yaml" = {
      text = cfg.config;
    };
    environment.etc."${cfg.configdir}/ton-global.config.json" = {
      text = builtins.readFile ./ton-global.config.json;
    };
    # Create systemd service
    systemd.services.ever-wallet-api = {
      enable = true;
      description = "Service that indexes transactions for Ever or Venom";
      after = ["network.target" cfg.dbPasswordFileService cfg.everSecretFileService cfg.everSaltFileService];
      wants = ["network.target" cfg.dbPasswordFileService cfg.everSecretFileService cfg.everSaltFileService];
      path = with pkgs; [ ];
      script = ''
        export DB_PASSWORD=$(cat ${cfg.dbPasswordFile} | xargs echo -n)
        export SECRET=$(cat ${cfg.everSecretFile} | xargs echo -n)
        export SALT=$(cat ${cfg.everSaltFile} | xargs echo -n)

        ${cfg.package}/bin/ton-wallet-api server \
          --config /etc/${cfg.configdir}/config.yaml \
          --global-config /etc/${cfg.configdir}/ton-global.config.json
      '';
      serviceConfig = {
          Restart = "always";
          RestartSec = 30;
          User = "ever-wallet-api";
          WorkingDirectory = "${cfg.datadir}";
        };
      wantedBy = ["multi-user.target"];
    };
    services.postgresql = {
        enable = true;
        # Ensure the database, user, and permissions always exist
        ensureDatabases = [ "${cfg.dbDatabase}" ];
        ensureUsers = [
            { 
                name = "${cfg.dbUser}";
                ensureDBOwnership = true;
            }
        ];
    };
    # Init folder for ever-wallet-api data
    system.activationScripts = {
      intever-wallet-api = {
        text = ''
          if [ ! -d "${cfg.datadir}" ]; then
            mkdir -p ${cfg.datadir}
            chown ever-wallet-api ${cfg.datadir}
          fi
          if [ ! -d "${cfg.configdir}" ]; then
            mkdir -p ${cfg.configdir}
          fi
          chown ton-wallet-api ${cfg.configdir}

          DB_PASSWORD=$(cat ${cfg.dbPasswordFile} | xargs echo -n)
          DATABASE_URL="postgresql://${cfg.dbUser}:''${DB_PASSWORD}@${cfg.dbHost}/${cfg.dbDatabase}"

          ${config.services.postgresql.package}/bin/psql -c "ALTER USER ${cfg.dbUser} PASSWORD '$DB_PASSWORD'"
          
          echo 'INFO: create database'
          ${pkgs.sqlx-cli}/bin/sqlx database create --database-url "$DATABASE_URL"

          echo 'INFO: apply database migration'
          ${pkgs.sqlx-cli}/bin/sqlx migrate run --database-url "$DATABASE_URL"
        '';
        deps = [];
      };
    };
  };
}

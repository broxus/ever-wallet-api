{ config, lib, pkgs, ... }:
with lib;  # use the functions from lib, such as mkIf
let
  # the values of the options set for the service by the user of the service
  cfg = config.services.tycho-wallet-api;
in {
  ##### interface. here we define the options that users of our service can specify
  options = {
    # the options for our service will be located under services.tycho-wallet-api
    services.tycho-wallet-api = {
      enable = mkOption {
        type = types.bool;
        default = true;
        description = ''
          Whether to enable tycho-wallet-api node by default.
        '';
      };
      package = mkOption {
        type = types.package;
        default = pkgs.tycho-wallet-api;
        defaultText = "pkgs.tycho-wallet-api";
        description = ''
          Which tycho-wallet-api package to use with the service.
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
        default = "Tycho";
        description = ''
          Which blockchain to use: Tycho, Venom
        '';
      };

      datadir = mkOption {
        type = types.str;
        default = "/var/lib/tycho-wallet-api";
        description = ''
          Path to service state on filesystem.
        '';
      };
      configdir = mkOption {
        type = types.str;
        default = "tycho-wallet-api";
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
        default = "tycho-wallet-api";
        description = ''
          Which username to use in PostgreSQL.
        '';
      };
      dbDatabase = mkOption {
        type = types.str;
        default = "tycho-wallet-api";
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
                # Root directory for tycho-wallet-api DB. Default: "./db"
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
                tycho_wallet_api:
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
        default = "/run/keys/tychowalletapidb";
        description = ''
          Location of file with password for RPC.
        '';
      };
      dbPasswordFileService = mkOption {
        type = types.str;
        default = "tychowalletapidb-key.service";
        description = ''
          Service that indicates that dbPasswordFile is ready.
        '';
      };
      tychoSecretFile = mkOption {
        type = types.str;
        default = "/run/keys/tychowalletapisecret";
        description = ''
          Location of file with secret for decrypting transactions.
        '';
      };
      tychoSecretFileService = mkOption {
        type = types.str;
        default = "tychowalletapisecret-key.service";
        description = ''
          Service that indicates that tychoSecretFile is ready.
        '';
      };
      tychoSaltFile = mkOption {
        type = types.str;
        default = "/run/keys/tychowalletapisalt";
        description = ''
          Location of file with salt for ???.
        '';
      };
      tychoSaltFileService = mkOption {
        type = types.str;
        default = "tychowalletapisalt-key.service";
        description = ''
          Service that indicates that tychoSaltFile is ready.
        '';
      };
    };
  };

  ##### implementation
  config = mkIf cfg.enable { # only apply the following settings if enabled
    # User to run the node
    users.users.tycho-wallet-api = {
      name = "tycho-wallet-api";
      group = "tycho-wallet-api";
      extraGroups = [ ];
      description = "tycho-wallet-api daemon user";
      home = cfg.datadir;
      isSystemUser = true;
    };
    users.groups.tycho-wallet-api = {};
    environment.etc."${cfg.configdir}/config.json" = {
      text = cfg.config;
    };
    environment.etc."${cfg.configdir}/global-config.json" = {
      text = builtins.readFile ./global-config.json;
    };
    # Create systemd service
    systemd.services.tycho-wallet-api = {
      enable = true;
      description = "Service that indexes transactions for Tycho or Venom";
      after = ["network.target" cfg.dbPasswordFileService cfg.tychoSecretFileService cfg.tychoSaltFileService];
      wants = ["network.target" cfg.dbPasswordFileService cfg.tychoSecretFileService cfg.tychoSaltFileService];
      path = with pkgs; [ ];
      script = ''
        export DB_PASSWORD=$(cat ${cfg.dbPasswordFile} | xargs echo -n)
        export SECRET=$(cat ${cfg.tychoSecretFile} | xargs echo -n)
        export SALT=$(cat ${cfg.tychoSaltFile} | xargs echo -n)

        ${cfg.package}/bin/tycho-wallet-api server \
          --config /etc/${cfg.configdir}/config.json \
          --global-config /etc/${cfg.configdir}/global-config.json \
          --keys /etc/${cfg.configdir}/keys.json
      '';
      serviceConfig = {
          Restart = "always";
          RestartSec = 30;
          User = "tycho-wallet-api";
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
    # Init folder for tycho-wallet-api data
    system.activationScripts = {
      inttycho-wallet-api = {
        text = ''
          if [ ! -d "${cfg.datadir}" ]; then
            mkdir -p ${cfg.datadir}
            chown tycho-wallet-api ${cfg.datadir}
          fi
          if [ ! -d "${cfg.configdir}" ]; then
            mkdir -p ${cfg.configdir}
          fi
          chown tycho-wallet-api ${cfg.configdir}

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

{config, lib, pkgs, ...}:
{
  imports = [
    ./module.nix
  ];
  config = {
    nixpkgs.overlays = [ 
        (import ./overlay.nix) 
    ];
    services.tycho-wallet-api = {
        enable = true;
        port = 7354;
        chain = "Tycho";
        dbPasswordFile = "/var/tychowalletapidb"; # fill it with password
        tychoSecretFile = "/var/tychowalletapisecret";
        tychoSaltFile = "/var/tychowalletapisalt";
        metricsHost = "0.0.0.0";
      };
    systemd.services = {
      tychowalletapidb-key = {
        enable = true;
        description = "Tycho wallet API password for PostgreSQL is provided";
        wantedBy = [ "network.target" ];
        serviceConfig.Type = "oneshot";
        serviceConfig.RemainAfterExit = true;
        script =
          ''
            echo "Tycho wallet API password for PostgreSQL is done"
          '';
      };
      tychowalletapisecret-key = {
        enable = true;
        description = "Tycho wallet encryption secret is provided";
        wantedBy = [ "network.target" ];
        serviceConfig.Type = "oneshot";
        serviceConfig.RemainAfterExit = true;
        script =
          ''
            echo "Tycho wallet encryption secret is done"
          '';
      };
      tychowalletapisalt-key = {
        enable = true;
        description = "Tycho wallet encryption salt is provided";
        wantedBy = [ "network.target" ];
        serviceConfig.Type = "oneshot";
        serviceConfig.RemainAfterExit = true;
        script =
          ''
            echo "Tycho wallet encryption salt is done"
          '';
      };
    };

  services.postgresql = {
        enable = true;    
        package = pkgs.postgresql_16; # pin it as other version deploy state to different directories
    };
  };
}

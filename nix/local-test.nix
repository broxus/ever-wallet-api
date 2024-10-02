{config, lib, pkgs, ...}:
{
  imports = [
    ./module.nix
  ];
  config = {
    nixpkgs.overlays = [ 
        (import ./overlay.nix) 
    ];
    services.ever-wallet-api = {
        enable = true;
        port = 7354;
        chain = "Everscale";
        dbPasswordFile = "/var/everwalletapidb"; # fill it with password
        everSecretFile = "/var/everwalletapisecret";
        everSaltFile = "/var/everwalletapisalt";
      };
    systemd.services = {
      everwalletapidb-key = {
        enable = true;
        description = "Ever wallet API password for PostgreSQL is provided";
        wantedBy = [ "network.target" ];
        serviceConfig.Type = "oneshot";
        serviceConfig.RemainAfterExit = true;
        script =
          ''
            echo "Ever wallet API password for PostgreSQL is done"
          '';
      };
      everwalletapisecret-key = {
        enable = true;
        description = "Ever wallet encryption secret is provided";
        wantedBy = [ "network.target" ];
        serviceConfig.Type = "oneshot";
        serviceConfig.RemainAfterExit = true;
        script =
          ''
            echo "Ever wallet encryption secret is done"
          '';
      };
      everwalletapisalt-key = {
        enable = true;
        description = "Ever wallet encryption salt is provided";
        wantedBy = [ "network.target" ];
        serviceConfig.Type = "oneshot";
        serviceConfig.RemainAfterExit = true;
        script =
          ''
            echo "Ever wallet encryption salt is done"
          '';
      };
    };

  services.postgresql = {
        enable = true;    
        package = pkgs.postgresql_16; # pin it as other version deploy state to different directories
    };
  };
}

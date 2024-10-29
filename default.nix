let
  sources = import ./nix/sources.nix;
  nixpkgs-mozilla = import sources.nixpkgs-mozilla;
  pkgs = import sources.nixpkgs {
    overlays =
      [
        nixpkgs-mozilla
        (self: super:
            let chan = self.rustChannelOf { rustToolchain = ./rust-toolchain.toml; };
            in {
              rustc = chan.rust;
              cargo = chan.rust;
            }
        )
      ];
  };
  naersk = pkgs.callPackage sources.naersk {};
  merged-openssl = pkgs.symlinkJoin { name = "merged-openssl"; paths = [ pkgs.openssl.out pkgs.openssl.dev ]; };
in
naersk.buildPackage {
  name = "ever-wallet-api";
  root = pkgs.lib.sourceFilesBySuffices ./. [".rs" ".toml" ".lock" ".html" ".css" ".png" ".sh" ".sql" ".proto" ".json"];
  buildInputs = with pkgs; [ sqlx-cli openssl pkgconfig clang llvm llvmPackages.libclang zlib cacert curl postgresql pkg-config ];
  LIBCLANG_PATH = "${pkgs.llvmPackages.libclang.lib}/lib";
  OPENSSL_DIR = "${merged-openssl}";
  PKG_CONFIG_PATH = "${pkgs.libgpg-error.dev}/lib/pkgconfig";
}
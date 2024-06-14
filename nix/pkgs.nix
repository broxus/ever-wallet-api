# To update nix-prefetch-git https://github.com/NixOS/nixpkgs
import ((import <nixpkgs> {}).fetchFromGitHub {
  owner = "NixOS";
  repo = "nixpkgs";
  rev = "3be88f5dc7f56dcc747cce2a641ca71e3f8b6890";
  sha256  = "sha256-54ArJ1HDqExPLcUs5xn5tQa+mu9f68ZegHzsbIZPxQY=";
})

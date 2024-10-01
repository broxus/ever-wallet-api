# To update nix-prefetch-git https://github.com/NixOS/nixpkgs
import ((import <nixpkgs> {}).fetchFromGitHub {
  owner = "NixOS";
  repo = "nixpkgs";
  rev = "3db11314908c41fcb4734a948a2a340c9c92ee68";
  sha256  = "sha256-R7aI1xw5yR71uotCe2TB4BJYoYngkTF3R9GQXOjb7Pk=";
})
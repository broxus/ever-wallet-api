# To update nix-prefetch-git https://github.com/NixOS/nixpkgs
import ((import <nixpkgs> {}).fetchFromGitHub {
  owner = "NixOS";
  repo = "nixpkgs";
  rev = "dd3a525eefe25af9bd19cac7f3b9962c9751f429" 
  sha256  = "sha256-QDRGRO8LWIrG6JMr0Y8CwwRr9f88PTH+Qu9Ez6VGR1I=";
})

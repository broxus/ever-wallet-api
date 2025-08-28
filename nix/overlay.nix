self: super: {
#   tycho-wallet-api = import (builtins.fetchGit {
#     url = "git@github.com:NCrashed/tycho-wallet-api.git";
#     ref = "master";
#     rev = "6a78807b10aaa622883dd0ef61acc60edcd4a23b";
#   });
    tycho-wallet-api = import ../default.nix;
}
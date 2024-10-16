self: super: {
#   ever-wallet-api = import (builtins.fetchGit {
#     url = "git@github.com:NCrashed/ever-wallet-api.git";
#     ref = "master";
#     rev = "6a78807b10aaa622883dd0ef61acc60edcd4a23b";
#   });
    ever-wallet-api = import ../default.nix;
}
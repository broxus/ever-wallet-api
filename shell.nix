with import <nixpkgs> {};
let 
  merged-openssl = symlinkJoin { name = "merged-openssl"; paths = [ openssl.out openssl.dev ]; };
  clang = pkgs.llvmPackages_14.clang;
  libclang = pkgs.llvmPackages_14.libclang;
  stdenv = pkgs.stdenv;
in stdenv.mkDerivation rec {
  name = "ever-wallet-api";
  env = buildEnv { name = name; paths = buildInputs; };

  buildInputs = [
    rustup
    openssl
    cmake
    clang
    libclang
    glibc.dev
    pkg-config
  ];

  shellHook = ''
    export OPENSSL_DIR="${merged-openssl}"
    export LIBCLANG_PATH="${libclang.lib}/lib"
    export LD_LIBRARY_PATH="${libclang.lib}/lib:$LD_LIBRARY_PATH"
    export NIX_CFLAGS_COMPILE="-I${glibc.dev}/include"
    export NIX_LDFLAGS="-L${glibc.dev}/lib"
    export C_INCLUDE_PATH="${glibc.dev}/include"
    export CPLUS_INCLUDE_PATH="${glibc.dev}/include"
    export CC=clang
    export CXX=clang++
    export CFLAGS="-O2"
    export CXXFLAGS="-O2"
    export RUSTFLAGS="-C opt-level=2"
  '';
}

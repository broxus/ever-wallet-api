with import <nixpkgs> {};

stdenv.mkDerivation rec {
  pname = "rocksdb";
  version = "6.22.1";

  src = fetchFromGitHub {
    owner = "facebook";
    repo = "rocksdb";
    rev = "v${version}";
    sha256 = "0lpw0xbr4lggxfs0fgz01kzaj2mpav7bcya1ih98wz7g8bqg0bz8";
  };

  nativeBuildInputs = [ cmake pkg-config ];

  buildInputs = [ zlib snappy gflags libgtest ];

  cmakeFlags = [
    "-DCMAKE_BUILD_TYPE=Release"
    "-DWITH_TESTS=OFF"
    "-DWITH_TOOLS=OFF"
    "-DWITH_BENCHMARK_TOOLS=OFF"
    "-DWITH_SNAPPY=ON"
    "-DWITH_ZLIB=ON"
  ];

  installPhase = ''
    mkdir -p $out/bin
    mkdir -p $out/lib
    mkdir -p $out/include
    cp -r librocksdb.* $out/lib/
    cp -r include/* $out/include/
  '';

  meta = with lib; {
    description = "A persistent key-value store for fast storage environments";
    homepage = "https://rocksdb.org/";
    license = licenses.apache20;
    platforms = platforms.linux;
  };
}

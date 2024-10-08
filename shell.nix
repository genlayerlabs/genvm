let
    pkgs = import (builtins.fetchTarball {
        url = "https://github.com/NixOS/nixpkgs/archive/05bbf675397d5366259409139039af8077d695ce.tar.gz";
    }) {
    };
in
pkgs.mkShellNoCC {
  packages = with pkgs; [
    ruby
    ninja
    (python312.withPackages (python-pkgs: with python-pkgs; [
      jsonnet
    ]))
    poetry
    curl
    git
    python3
    zip
    unzip
    gnutar
    tree
    vim
    rustup
    mold
    clang
    openssl
    pkg-config
    zlib
    wabt
  ];
  LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath [ pkgs.openssl pkgs.zlib ];
}

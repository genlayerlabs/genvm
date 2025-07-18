{
	inputs = {
		nixpkgs.url = "github:NixOS/nixpkgs/nixos-25.05";
		systems = {
			url = "github:nix-systems/default";
			inputs.nixpkgs.follows = "nixpkgs";
		};
		flake-utils = {
			url = "github:numtide/flake-utils";
			inputs.systems.follows = "systems";
			inputs.nixpkgs.follows = "nixpkgs";
		};
	};

	outputs = inputs@{ self, nixpkgs, flake-utils, ... }:
		flake-utils.lib.eachDefaultSystem
			(system:
				let
					pkgs = nixpkgs.legacyPackages.${system};
				in
				{
					devShells.default = pkgs.mkShell {
						packages = with pkgs; [
							ninja
							ruby

							rustup

							curl

							python312
							python312Packages.jsonnet
							poetry
							pre-commit

							xz
							zlib
							glibc
							aflplusplus

							wabt

							nodejs
							nodePackages.npm

							glibc

							mermaid-cli
							ripgrep
						];

						shellHook = ''
							export NPM_CONFIG_PREFIX="$(pwd)/.direnv/npm-prefix"
							export PATH="$(pwd)/tools/ya-build:$(pwd)/tools/git-third-party:$PATH"
							export LD_LIBRARY_PATH="${toString pkgs.xz.out}/lib:${toString pkgs.zlib.out}/lib:${toString pkgs.stdenv.cc.cc.lib}/lib:${toString pkgs.glibc}/lib:$LD_LIBRARY_PATH"
						'';
					};
				}
			);
}

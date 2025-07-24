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
					pkgs = import nixpkgs {
						inherit system;
						config.allowUnfreePredicate = pkg:
							builtins.elem (pkgs.lib.getName pkg) [
								"vscode"
							];
					};
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

							(vscode.overrideAttrs (oldAttrs: rec {
								src = (builtins.fetchTarball {
									url = "https://update.code.visualstudio.com/1.102.2/linux-x64/stable";
									sha256 = "sha256:1h7g8gng7yqzjp90r835mhjbswykynjsys09d3z2llbwqdqj7nvd";
								});
								version = "1.102.2";
							}))
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

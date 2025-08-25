# we have following targets:
# - x86_64-linux-musl
# - aarch64-linux-musl
# - aarch64-macos
# - universal

# runners are universal target
# lib, modules and executor are platform-dependent
# each platform is going to have same layout
# full installation is going to be a merge of platform-specific and universal

{
	inputs = {
		nixpkgs.url = "github:NixOS/nixpkgs/2b4230bf03deb33103947e2528cac2ed516c5c89";
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

	outputs = { self, nixpkgs }:
		let
			genvm-release =
				let
					pkgs = import nixpkgs {
						system = "x86_64-linux";
					};

					lib = pkgs.lib;

					args = import ./support {
						inherit pkgs;
						root-src = self;
					} // {
						inherit components;

						build-config = {
							executor-version = "0.1.18";
							repo-url = "https://github.com/genlayerlabs/genvm.git";
							head-revision = "aboba";
						};
					};

					components = args.merge-components [
						(import ./libs args)
						(import ./modules args)
						(import ./executor args)
						(import ./runners/release.nix args)
						(import ./runners/all args)
					];

					merge-all-for-platform = platform:
						let
							for-platform = components.${platform};
							names = builtins.attrNames for-platform;
							just-derivations = builtins.attrValues for-platform;
						in
							pkgs.stdenvNoCC.mkDerivation {
								name = "genvm-${platform}";

								srcs = just-derivations;

								dontUnpack = true;
								dontConfigure = true;
								dontBuild = true;
								dontFixup = true;

								installPhase = ''
									mkdir -p $out
									for src in $srcs; do
										cp --no-preserve=ownership -r $src/. $out/.
										chmod -R u+w $out
									done
								'';
							};
				in {
					inherit components;

					all-for-platform = builtins.mapAttrs (platform: sub: merge-all-for-platform platform) components;
				};

				for-systems =
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
								devShells.minimal = pkgs.mkShell {
									packages = with pkgs; [
										curl
										ninja

										ruby

										python312
										python312Packages.jsonnet
										poetry
										pre-commit

										xz
										zlib
										glibc
										aflplusplus

										wabt

										glibc
									];

									shellHook = ''
										export PATH="$(pwd)/tools/git-third-party:$PATH"
										export LD_LIBRARY_PATH="${toString pkgs.xz.out}/lib:${toString pkgs.zlib.out}/lib:${toString pkgs.stdenv.cc.cc.lib}/lib:${toString pkgs.glibc}/lib:$LD_LIBRARY_PATH"
									'';
								};
							}
						);
			in
			for-systems // genvm-release;
}

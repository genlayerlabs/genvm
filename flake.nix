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

	outputs = { self, nixpkgs, flake-utils, systems }:
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

						build-config = builtins.fromJSON (builtins.readFile ./flake-config.json);
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
								};
								packages-0 = with pkgs; [ bash xz zlib glibc git python312 coreutils which ];
								packages-lint = with pkgs; [ pre-commit ];
								packages-rust = [ (import ./support/rust.nix { inherit pkgs system; withLinters = true; withZig = false; }) ];
								packages-debug-test = with pkgs; [
									(pkgs.ninja.overrideAttrs (old: {
										postPatch = old.postPatch + ''
											substituteInPlace src/subprocess-posix.cc \
												--replace '"/bin/sh"' '"${pkgs.bash}/bin/bash"'
										'';
									}))
									ruby
									gcc

									aflplusplus
									python312Packages.jsonnet
									wabt
								];
								packages-py-test = with pkgs; [
									python312
									poetry
								];
								shell-hook-base = ''
									export PATH="$(pwd)/tools/git-third-party:$PATH"
									export LD_LIBRARY_PATH="${toString pkgs.xz.out}/lib:${toString pkgs.zlib.out}/lib:${toString pkgs.stdenv.cc.cc.lib}/lib:${toString pkgs.glibc}/lib:$LD_LIBRARY_PATH"
									export LLVM_PROFILE_FILE=/dev/null
								'';
							in
							{
								devShells.py-test = pkgs.mkShell {
									packages = packages-py-test;
									shellHook = shell-hook-base;
								};
								devShells.initial-check = pkgs.mkShell {
									packages = packages-0 ++ packages-rust ++ packages-lint;
									shellHook = shell-hook-base;
								};
								devShells.rust-test = pkgs.mkShell {
									packages = packages-0 ++ packages-debug-test ++ packages-rust;
									shellHook = shell-hook-base;
								};
								devShells.full = pkgs.mkShell {
									packages = packages-0 ++ packages-debug-test ++ packages-py-test ++ packages-rust ++ packages-lint;
									shellHook = shell-hook-base;
								};
							}
						);
			in
			for-systems // genvm-release;
}

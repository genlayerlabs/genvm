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
		};
		flake-utils = {
			url = "github:numtide/flake-utils";
			inputs.systems.follows = "systems";
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
						(import ./runners/support/all args)
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

								custom-rust = import ./support/rust.nix { inherit pkgs system; withLinters = true; withZig = false; };
								custom-rust-builder = import ./support/compile-rust.nix {
									inherit pkgs system;
									zig = import ./support/zig.nix { inherit pkgs system; };
								};

								custom-cargo-afl = custom-rust-builder rec {
									name = "cargo-afl";
									version = "0.15.18";
									src = pkgs.fetchzip {
										url = "https://crates.io/api/v1/crates/cargo-afl/0.15.18/download";
										hash = "sha256-6ti50bwE4bLwIyR76bMt/Vn6Nwqu9n0IKdVuDdYkiHg=";
										extension = ".tar.gz";
										name = "cargo-afl-0.15.18.tar.gz";
									};

									target = system;

									cargoLock.lockFile = "${src}/Cargo.lock";

									nativeBuildInputs = [ pkgs.gnumake pkgs.makeWrapper ];

									postBuild = ''
										XDG_DATA_HOME="$out/data" ./target/*/release/cargo-afl afl config --build --verbose
									'';

									installPhase = ''
										mkdir -p $out/bin
										cp target/__out $out/bin/cargo-afl
										wrapProgram $out/bin/cargo-afl \
											--set XDG_DATA_HOME "$out/data"
									'';
								};

								packages-0 = with pkgs; [ bash xz zlib git python312 coreutils which jq stdenv.cc glibc nix ];
								packages-lint = with pkgs; [ pre-commit ];
								packages-rust = [ custom-rust ];
								packages-debug-test = with pkgs; [
									(pkgs.ninja.overrideAttrs (old: {
										postPatch = old.postPatch + ''
											substituteInPlace src/subprocess-posix.cc \
												--replace '"/bin/sh"' '"${pkgs.bash}/bin/bash"'
										'';
									}))
									ruby
									gcc

									custom-cargo-afl
									llvmPackages.libllvm

									python312Packages.jsonnet
									pkgs.python312Packages.aiohttp
									wabt
								];
								packages-py-test = with pkgs; [
									# aflplusplus # currently we don't run fuzzing on CI
									python312
									poetry
								];
								shell-hook-base = ''
									export PATH="$(pwd)/tools/git-third-party:$PATH"
									export LD_LIBRARY_PATH="${toString pkgs.xz.out}/lib:${toString pkgs.zlib.out}/lib:${pkgs.stdenv.cc.cc.lib}/lib:${toString pkgs.glibc}/lib:$LD_LIBRARY_PATH"
									export LLVM_PROFILE_FILE=/dev/null
								'';
							in
							{
								devShells.py-test = pkgs.mkShell {
									packages = packages-py-test ++ [ pkgs.ruby ];
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
								devShells.mock-tests = pkgs.mkShell {
									packages = packages-0 ++ [
										pkgs.python312
										pkgs.python312Packages.jsonnet
										pkgs.python312Packages.aiohttp
										pkgs.wabt
									];
									shellHook = shell-hook-base;
								};
								devShells.full = pkgs.mkShell {
									packages = packages-0 ++ packages-debug-test ++ packages-py-test ++ packages-rust ++ packages-lint;
									shellHook = shell-hook-base;
								};
								devShells.check-qemu = pkgs.mkShell {
									packages = packages-0 ++ [ pkgs.qemu ];
									shellHook = shell-hook-base;
								};
							}
						);
			in
			for-systems // genvm-release;
}

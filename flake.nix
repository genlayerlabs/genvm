{
	inputs = {
		nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
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
			for-systems =
				flake-utils.lib.eachDefaultSystem
					(system:
						let
							pkgs = import nixpkgs {
								inherit system;
							};

							ya-test-runner = pkgs.python312Packages.buildPythonApplication {
								pname = "ya-test-runner";
								version = "0.1.0";
								pyproject = true;
								src = ./tools/ya-test-runner;
								build-system = [ pkgs.python312Packages.poetry-core ];
								dependencies = with pkgs.python312Packages; [ aiohttp jsonnet ];
								doCheck = false;
							};

							deps = import ./runners/support/deps/fetch-deps.nix { inherit pkgs; };

							custom-rust = import ./support/rust.nix { inherit pkgs deps system; withLinters = true; withZig = false; withWasi = true; };
							custom-rust-builder = import ./support/compile-rust.nix {
								inherit pkgs system deps;
								zig = import ./support/zig.nix { inherit pkgs deps system; };
							};

							custom-cargo-afl = custom-rust-builder rec {
								name = "cargo-afl";
								version = "0.17.1";
								src = deps."cargo-afl-0.17.1";

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
								ya-test-runner
							];
							packages-gen-docs = with pkgs; [
								lua-language-server
								mermaid-cli
							];
							packages-py-test = with pkgs; [
								# aflplusplus # currently we don't run fuzzing on CI
								python312
								poetry
							];
							shell-hook-base = ''
								export PATH="$(pwd)/tools/git-third-party:$PATH"
								export CARGO_LD_LIBRARY_PATH="${toString pkgs.xz.out}/lib:${toString pkgs.zlib.out}/lib:${pkgs.stdenv.cc.cc.lib}/lib:${toString pkgs.glibc}/lib"
								export LLVM_PROFILE_FILE=/dev/null
							'';

							release-args = import ./support {
								inherit pkgs system;
								root-src = self;
							} // {
								host-system = system;
								host-system-as-genvm = {
									"x86_64-linux" = "amd64-linux";
									"aarch64-linux" = "arm64-linux";
									"aarch64-darwin" = "arm64-macos";
								}."${system}";

								build-config = builtins.fromJSON (builtins.readFile ./flake-config.json);
							};

							compiled-libs = import ./libs release-args;

							debug-runners = import ./runners/support/views/debug-build.nix {
								inherit pkgs;
								host-system = system;
							};

							runners-args = {
								inherit pkgs deps;
								host-system = system;
								build-config = builtins.fromJSON (builtins.readFile ./flake-config.json);
							};

							runners-list = import ./runners/support/versions/all.nix runners-args;

							runners-universal-set = (import ./runners/support/views/all-universal.nix runners-args).universal;

							runners-all = pkgs.symlinkJoin {
								name = "genvm-runners-all";
								paths = builtins.attrValues runners-universal-set;
							};
						in
						{
							packages =
								{
									inherit debug-runners ya-test-runner;
									runners = runners-list;
									inherit runners-all;
								}
								// (import ./executor (release-args // { inherit compiled-libs; }))
								// (import ./modules (release-args // { inherit compiled-libs; }))
							;

							devShells.py-test = pkgs.mkShell {
								packages = packages-py-test ++ [ pkgs.ruby ];
								shellHook = shell-hook-base + ''
									export LD_LIBRARY_PATH="${pkgs.stdenv.cc.cc.lib}/lib:${toString pkgs.zlib.out}/lib:''${LD_LIBRARY_PATH:-}"
								'';
							};
							devShells.gen-docs = pkgs.mkShell {
								packages = packages-py-test ++ packages-gen-docs ++ [ pkgs.ruby ];
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
								packages = packages-0 ++ packages-rust ++ packages-debug-test;
								shellHook = shell-hook-base;
							};
							devShells.full = pkgs.mkShell {
								packages =
									packages-0 ++
									packages-debug-test ++
									packages-py-test ++
									packages-rust ++
									packages-lint ++
									packages-gen-docs ++
									[ pkgs.nodejs ];
								shellHook = shell-hook-base;
							};
							devShells.check-qemu = pkgs.mkShell {
								packages = packages-0 ++ [ pkgs.qemu ];
								shellHook = shell-hook-base;
							};
						}
					);
		in
		for-systems;
}

{
	inputs = {
		nixpkgs.url = "github:NixOS/nixpkgs/2b4230bf03deb33103947e2528cac2ed516c5c89";
	};
	outputs = inputs@{ self, nixpkgs, ... }:
		let
			pkgs = import nixpkgs {
				system = "x86_64-linux";
			};

			lib = pkgs.lib;

			nixHashes = {
				# pkgs.lib.fakeHash
				genvm-cpython-ext = "sha256-sccECJ7o8fVCYM+7ngNZTl9enGfJWoe1gapeNNyG0H8=";
				cpython = "sha256-Nj1qtLHu0er4PcXIVXBawBKP7pPTDfrYHY9zA1RnQwc=";
				topmost = "sha256-bzgiTVmrdfjRnTNv63Ul6QfZd9PokLPtqhJ1vSgHb6A=";
			};

			wasmShell = (import ./envs/wasm.nix args);
			pyShell = (import ./envs/py.nix args);
			rustShell = (import ./envs/rs.nix args);

			tools = (import ./tools args);

			runnerHashes = builtins.fromJSON (builtins.readFile ./hashes.json);

			args = {
				inherit pkgs wasmShell pyShell rustShell runnerHashes tools nixHashes;
				lib = pkgs.lib;
			};
		in {
			genvm-runners-all = import ./trg args;

			genvm-make-runner = tools.genvm-make-runner;
			genvm-py-precompile = tools.genvm-py-precompile;
		};
}

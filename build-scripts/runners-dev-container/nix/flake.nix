{
	inputs = {
		nixpkgs.url = "github:NixOS/nixpkgs/2b4230bf03deb33103947e2528cac2ed516c5c89";
		make-shell = {
			url = "github:nicknovitski/make-shell";
			inputs.nixpkgs.follows = "nixpkgs";
		};
	};
	outputs = inputs@{ self, nixpkgs, make-shell, ... }:
		let
			pkgs = import nixpkgs {
				system = "x86_64-linux";
				overlays = [make-shell.overlays.default];
			};

			wasmShell = (import ./envs/wasm.nix args);
			pyShell = (import ./envs/py.nix args);

			args = {
				inherit pkgs wasmShell pyShell;
				lib = pkgs.lib;
			};
		in {
			devShells.x86_64-linux = {
				genvmWasm = wasmShell;
			};

			packages.x86_64-linux.genvm-all = (import ./trg args);
		};
}

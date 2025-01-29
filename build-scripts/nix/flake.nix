{
	inputs = {
		nixpkgs.url = "github:NixOS/nixpkgs/release-24.05";
	};
	outputs = { nixpkgs, ... }@inputs:
		let
			pkgs = import nixpkgs { system = "x86_64-linux"; };
		in
		{
			devShells.x86_64-linux = {
				foo = pkgs.mkShell {
					buildInputs = [
						pkgs.hello
					];
				};
			};
		};
}

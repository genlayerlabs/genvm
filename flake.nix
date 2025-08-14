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
	};

	outputs = { self, nixpkgs }:
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
				};
			};

			components = args.merge-components [
				(import ./libs args)
				(import ./modules args)
				(import ./executor args)
				(import ./runners/release.nix args)
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
}

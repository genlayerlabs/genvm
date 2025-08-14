{ pkgs
, build-config
, ...
}:
let
	latest =  builtins.toFile "latest.json" (builtins.toJSON (import ./latest.nix));
	subpath = "executor/${build-config.executor-version}/data/";
in {
	universal = {
		runners-latest = pkgs.stdenvNoCC.mkDerivation rec {
			name = "genvm-runners-latest";

			dontUnpack = true;
			dontConfigure = true;
			dontBuild = true;
			dontFixup = true;

			installPhase = ''
				mkdir -p $out/${subpath}
				cp ${latest} $out/${subpath}/latest.json
			'';
		};
	};
}

{ pkgs
, ...
}@args:
{
	universal = builtins.foldl' (acc: x: acc // {
		"${x.uid}" = pkgs.stdenvNoCC.mkDerivation {
			name = "genvm-runner-${x.uid}";
			src = x.derivation;
			dontUnpack = true;
			dontConfigure = true;
			dontBuild = true;
			dontFixup = true;
			installPhase = let
				hash32 = builtins.head (builtins.tail (builtins.match "([^:]+):(.*)" x.uid));
				result-path = "runners/${x.id}/${builtins.substring 0 2 hash32}/${builtins.substring 2 50 hash32}.tar";
			in ''
				mkdir -p $out/$(dirname -- ${result-path})
				cp ${x.derivation} "$out/${result-path}"
			'';
		};
	}) {} (import ../versions/all.nix args);
}

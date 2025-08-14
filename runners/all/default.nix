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
			installPhase = ''
				mkdir -p "$out/runners/${x.id}"
				cp ${x.derivation} "$out/runners/${x.id}/${builtins.convertHash { hash = x.hash; toHashFormat = "nix32"; }}.tar"
			'';
		};
	}) {} (import ./all.nix args);
}

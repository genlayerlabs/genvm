{ pkgs
, pythonObjs
, stdenvNoCC
, runnersLib
, lib
, ...
}:
let
	genlayer_c = pkgs.writeText "genlayer.c" (builtins.readFile ./genlayer.c);
	extraObj = stdenvNoCC.mkDerivation {
		name = "genvm-cpython-mod-genlayer-objs";
		outputHashMode = "recursive";
		outputHash = "sha256-g/Xx9ZN2K4qSCxHnGShFJJwt4w82eCsRLv8a2KfmdXg=";

		deps = [ genlayer_c ];

		src = pythonObjs;

		phases = [ "unpackPhase" "buildPhase" "installPhase" ];

		nativeBuildInputs = [
			runnersLib.wasi-sdk.package
		];

		postUnpack = ''
			cp "${genlayer_c}" ./genlayer.c
		'';

		buildPhase = ''
			${runnersLib.wasi-sdk.env.CC} ${runnersLib.wasi-sdk.env.CFLAGS} -Wall -Wextra -Wpedantic -Werror -Wno-unused-parameter -I ./include/python3.13 -c -o genlayer.o ../genlayer.c
		'';

		installPhase = ''
			mkdir -p "$out/obj"
			cp ./genlayer.o "$out/obj/"
		'';
	};
in {
	runners = [];
	extraObjs = [extraObj];

	setupLines = [
		"_genlayer_wasi"
	];
}

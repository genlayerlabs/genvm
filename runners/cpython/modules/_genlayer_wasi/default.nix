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
		outputHash = "sha256-7Vvxw4A2TlSDUOzkWga1i0X09xKs3ro/qYpIlffs7ks=";

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

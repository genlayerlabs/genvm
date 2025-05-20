{ pkgs
, pythonObjs
, stdenvNoCC
, runnersLib
, lib
, ...
}:
let
	h_config_private = pkgs.writeText "opj_config_private.h" (builtins.readFile ./opj_config_private.h);
	h_config = pkgs.writeText "opj_config.h" (builtins.readFile ./opj_config.h);
	c_opj_clock = pkgs.writeText "opj_clock.c" (builtins.readFile ./opj_clock.c);

	extra_c_files = [
		"decode"
		"encode"
		"map"
		"display"
		"outline"
		"path"

		"_imaging"
		"_webp"
		#"_imagingft"
		"_imagingmath"
		"_imagingmorph"
	];

	makefile_in = pkgs.writeText "Makefile.in" (builtins.readFile ./Makefile.in);

	extra_c_files_str = builtins.concatStringsSep " " (builtins.map (x: "pillow-src/src/" + x + ".c") extra_c_files);

	extraObj = stdenvNoCC.mkDerivation {
		name = "genvm-cpython-mod-pil-objs";
		outputHashMode = "recursive";
		outputHash = "sha256-hGTSKu3MU+UE4zFR4ROUUYMc8ATts2bEjs2Y3L1hc28=";

		srcs = [
			pythonObjs

			(builtins.fetchGit {
				url = "https://github.com/uclouvain/openjpeg.git";
				rev = "e7453e398b110891778d8da19209792c69ca7169";
				name = "openjpeg-src";
				shallow = true;
			})

			(builtins.fetchGit {
				url = "https://github.com/python-pillow/Pillow.git";
				rev = "3c71559804e661a5f727e2007a5be51f26d9af27";
				name = "pillow-src";
				shallow = true;
			})

			(builtins.fetchGit {
				url = "https://github.com/webmproject/libwebp.git";
				rev = "0cd0b7a7013723985156989f0772e3cb8c4ce49f";
				name = "libwebp-src";
				shallow = true;
			})
		];

		sourceRoot = ".";

		phases = [ "unpackPhase" "buildPhase" "installPhase" ];

		postUnpack = ''
			cp ${h_config_private} openjpeg-src/src/lib/openjp2/opj_config_private.h
			cp ${h_config} openjpeg-src/src/lib/openjp2/opj_config.h
			cp ${c_opj_clock} openjpeg-src/src/lib/openjp2/opj_clock.c
		'';

		nativeBuildInputs = [
			runnersLib.wasi-sdk.package
			pkgs.perl
		];

		buildPhase = ''
			### prelude ###
			mkdir -p _incl
			mkdir -p obj

			### openjpeg ###
			SRCS="SRCS ="
			for f in $(find openjpeg-src/src/lib/openjp2/ -name '*.c' -and -not -name '*bench_*' -and -not -name '*test_*' -and -not -name '*_manager.c' -and -not -name 't1_*')
			do
				SRCS="$SRCS $f"
			done

			echo "$SRCS" > ./Makefile
			cat ${makefile_in} >> ./Makefile

			${runnersLib.wasi-sdk.env-str} make -j all

			cp openjpeg-src/src/lib/openjp2/opj_config.h openjpeg-src/src/lib/openjp2/openjpeg.h _incl

			### webp ###
			cp -r libwebp-src/src/webp _incl/

			SRCS="SRCS ="
			for f in $(find libwebp-src/src/ libwebp-src/sharpyuv -name '*.c')
			do
				SRCS="$SRCS $f"
			done

			echo "$SRCS" > ./Makefile
			cat ${makefile_in} >> ./Makefile

			${runnersLib.wasi-sdk.env-str} CFLAGS="$CFLAGS -Ilibwebp-src" make -j all

			### pillow ###

			SRCS="SRCS ="
			for f in $(cat <(find pillow-src/src/libImaging -name '*.c') <(echo ${extra_c_files_str}))
			do
				SRCS="$SRCS $f"
			done

			perl -pe 's/(ImagingSection(Enter|Leave))/_$1/g' -i pillow-src/src/_webp.c

			echo "$SRCS" > ./Makefile
			cat ${makefile_in} >> ./Makefile

			${runnersLib.wasi-sdk.env-str} \
				CFLAGS="$CFLAGS -DHAVE_LIBZ -DHAVE_OPENJPEG -isystem _incl '-DPILLOW_VERSION=\"11.3.0.dev0\"' -I/build/${pythonObjs.name}/include/python3.13/ -I/build/${pythonObjs.name}/include/" \
				make -j all
		'';

		installPhase = ''
			mkdir -p "$out/obj"

			mkdir -p "$out/py/libs"

			cp -r ./obj/. "$out/obj"
			cp -r ./pillow-src/src/PIL "$out/py/libs"

			echo 'from _imaging import *' >> "$out/py/libs/PIL/_imaging.py"
			echo 'from _imagingmath import *' >> "$out/py/libs/PIL/_imagingmath.py"
			echo 'from _imagingmorph import *' >> "$out/py/libs/PIL/_imagingmorph.py"
			echo 'from _webp import *' >> "$out/py/libs/PIL/_webp.py"
		'';
	};
in {
	runners = [];
	extraObjs = [extraObj];

	setupLines = [
		"_imaging"
		"_imagingmath"
		"_imagingmorph"
		"_webp"
		#"_imagingcms"
		# "_imagingft"
	];
}

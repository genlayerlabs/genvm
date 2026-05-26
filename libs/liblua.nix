{ name-target
, pkgs
, zig
, lua-src
}:
let
	version = "5.3";

	isMacos = name-target == "arm64-macos";

	outSuffix = if isMacos then "dylib" else "so";
in pkgs.stdenvNoCC.mkDerivation {
	name = "liblua-${name-target}";

	inherit version;

	src = lua-src;

	nativeBuildInputs = [ zig ] ++ (if isMacos then [ pkgs.pkgsCross.aarch64-darwin.buildPackages.stdenv.cc ] else []);

	doNotConfigure = true;

	buildPhase = ''
		set -e

		export SOURCE_DATE_EPOCH=1609459200

		for i in ./*.c ; do
			zig-cc-${name-target} ${if isMacos then "-g0" else ""} -O2 -fPIC -I. -fdebug-prefix-map=${toString zig}=/zig -no-canonical-prefixes -c "$i" -o "$i.o"
		done

		ls *.o | sort | xargs zig-cc-${name-target} --verbose -O2 -fPIC -shared -o liblua.${outSuffix}
	'';

	installPhase = ''
		mkdir -p "$out/lib"
		echo "${toString zig}"
		cp liblua.${outSuffix} "$out/lib/liblua.${outSuffix}"
	'';
}

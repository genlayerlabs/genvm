{ name-target
, pkgs
, zig
, deps
, lua-src
}:
let
	lsqlite3-src = deps."lsqlite3-0.9.6";

	isMacos = name-target == "arm64-macos";

	outSuffix = if isMacos then "dylib" else "so";
in pkgs.stdenvNoCC.mkDerivation {
	name = "lsqlite3-${name-target}";

	src = lsqlite3-src;

	nativeBuildInputs = [ zig ] ++ (if isMacos then [ pkgs.pkgsCross.aarch64-darwin.buildPackages.stdenv.cc ] else []);

	dontConfigure = true;

	buildPhase = ''
		set -e

		export SOURCE_DATE_EPOCH=1609459200

		zig-cc-${name-target} ${if isMacos then "-g0" else ""} -O2 -fPIC -DSQLITE_CORE -I. -fdebug-prefix-map=${toString zig}=/zig -no-canonical-prefixes -c sqlite3.c -o sqlite3.o
		zig-cc-${name-target} ${if isMacos then "-g0" else ""} -O2 -fPIC -I${lua-src} -I. -fdebug-prefix-map=${toString zig}=/zig -no-canonical-prefixes -c lsqlite3.c -o lsqlite3.o

		zig-cc-${name-target} -O2 -fPIC -shared -o lsqlite3.${outSuffix} sqlite3.o lsqlite3.o
	'';

	installPhase = ''
		mkdir -p "$out/lib/genvm-lua"
		cp lsqlite3.${outSuffix} "$out/lib/genvm-lua/lsqlite3.${outSuffix}"
	'';
}

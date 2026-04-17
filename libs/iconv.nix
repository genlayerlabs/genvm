{ pkgs
, zig
, deps
}:
let
	iconv-src = deps."libiconv";
in pkgs.stdenvNoCC.mkDerivation {
	name = "libiconv";

	src = iconv-src;

	nativeBuildInputs = [ zig pkgs.coreutils ];

	configurePhase = ''
		CC=zig-cc-arm64-macos \
			LD=zig-cc-arm64-macos \
			AR="${zig}/zig ar" \
			CFLAGS="-O2" \
			./configure --host=aarch64-apple-darwin --enable-shared=yes
	'';

	buildPhase = ''
		make -j
	'';

	installPhase = ''
		mkdir -p "$out/lib"
		cp lib/.libs/libiconv.dylib "$out/lib/libiconv.dylib"
	'';
}

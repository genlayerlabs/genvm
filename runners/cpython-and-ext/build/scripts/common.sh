set -ex

export PKG_CONFIG_LIBDIR="$WASM32_WASI_ROOT/lib/pkgconfig"
DETERMINISTIC_C_FLAGS="-Wno-builtin-macro-redefined -D__TIME__='\"00:42:42\"' -D__DATE__='\"Jan_24_2024\"'"

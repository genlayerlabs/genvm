set -e
AR_SCRIPT="CREATE libffi.a"

make wasm32-unknown-wasip1/include/ffi.h

for i in stub_ffi.c src/closures.c src/prep_cif.c src/tramp.c src/debug.c src/raw_api.c src/types.c
do
	FNAME="$(basename "$i")"
	clang \
		$(cat .cflags) \
		-o "$FNAME.o" \
		-c "$i"
	AR_SCRIPT="$AR_SCRIPT"$'\n'"ADDMOD $FNAME.o"
done

AR_SCRIPT="$AR_SCRIPT"$'\n'"SAVE"
AR_SCRIPT="$AR_SCRIPT"$'\n'"END"

echo "$AR_SCRIPT" | ar -M

#!/usr/bin/env bash

TARGET=""
EXECUTOR_VERSION=""
EXTRA_BUNDLE=""

while [[ $# -gt 0 ]]; do
	case $1 in
		--target)
			TARGET="$2"
			shift 2
			;;
		--target=*)
			TARGET="${1#*=}"
			shift
			;;
		--executor-version)
			EXECUTOR_VERSION="$2"
			shift 2
			;;
		--executor-version=*)
			EXECUTOR_VERSION="${1#*=}"
			shift
			;;
		--extra-bundle)
			EXTRA_BUNDLE="$2"
			EXTRA_BUNDLE="$(readlink -f "$EXTRA_BUNDLE")"
			shift 2
			;;
		--extra-bundle=*)
			EXTRA_BUNDLE="${1#*=}"
			EXTRA_BUNDLE="$(readlink -f "$EXTRA_BUNDLE")"
			shift
			;;
		-h|--help)
			echo "Usage: $0 [--target TARGET] [--executor-version VERSION]"
			echo "  --target TARGET              Specify the target platform (default: universal)"
			echo "  --executor-version VERSION   Specify the executor version"
			exit 0
			;;
		*)
			echo "Unknown option: $1" >&2
			exit 1
			;;
	esac
done

# Validate arguments
if [[ -n "$TARGET" && -z "$TARGET" ]]; then
	echo "Error: --target cannot be empty" >&2
	exit 1
fi

if [[ -n "$EXECUTOR_VERSION" && -z "$EXECUTOR_VERSION" ]]; then
	echo "Error: --executor-version cannot be empty" >&2
	exit 1
fi

HEAD_REVISION=$(git rev-parse HEAD)

echo "\$TARGET = $TARGET"
echo "\$HEAD_REVISION = $HEAD_REVISION"
echo "\$EXECUTOR_VERSION = $EXECUTOR_VERSION"

SCRIPT_DIR=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )
source "$SCRIPT_DIR/_common.sh"

cat <<EOF > flake-config.json
{
  "executor-version": "$EXECUTOR_VERSION",
  "repo-url": "https://github.com/genlayerlabs/genvm.git",
  "head-revision": "$HEAD_REVISION"
}
EOF

mkdir -p build
nix build -o build/out-$TARGET -v -L .#all-for-platform.$TARGET --show-trace

PREV=$(readlink -f .)
pushd build/out-$TARGET
find . -type f -print0 | sort -z | \
	xargs -0 tar --transform 's,^\./,,' --mode=ug+w -cf - | \
	cat - $(if [ "$EXTRA_BUNDLE" != "" ]; then echo "$EXTRA_BUNDLE"; fi) | \
	xz -9 > "$PREV/build/genvm-$TARGET.tar.xz"
popd

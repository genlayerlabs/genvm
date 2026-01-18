set -ex

SCRIPT_DIR=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )

cd "$SCRIPT_DIR"

docker build -q . 2>&1 > /dev/null || docker build . --progress=plain

SHA=$(docker build -q .)
docker run --rm -d -p 4444:4444 "$SHA"

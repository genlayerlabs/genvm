#!/usr/bin/env bash

set -e

FILTER='.*'
SHOW_HELP=false

FUZZ_TIMEOUT=30

PRECOMPILE=false

show_help() {
    cat << EOF
Usage: $0 [OPTIONS]

This must be ran after ./configure.rb

OPTIONS:
    --help              Show this help message
    --filter REGEX      Set filter regex
    --fuzz-timeout SECS Seconds for which to run fuzzing
    --precompile        Run precompile for genvm

Examples:
    $0 --filter '.*'
    $0 --help

to run it you need following packages:
    - cargo-afl

rustup components:
    - llvm-tools-preview
EOF
}

while [[ $# -gt 0 ]]; do
    case $1 in
        --help)
            SHOW_HELP=true
            shift
            ;;
        --filter)
            if [[ -n $2 && $2 != --* ]]; then
                FILTER="$2"
                shift 2
            else
                echo "Error: --filter requires an argument" >&2
                exit 1
            fi
            ;;
        --fuzz-timeout)
            if [[ -n $2 && $2 != --* ]]; then
                FUZZ_TIMEOUT="$2"
                shift 2
            else
                echo "Error: --filter requires an argument" >&2
                exit 1
            fi
            ;;
        --precompile)
            PRECOMPILE=true
            shift
        ;;
        *)
            echo "Error: Unknown option $1" >&2
            show_help
            exit 1
            ;;
    esac
done

if [[ $SHOW_HELP == true ]]; then
    show_help
    exit 0
fi

SCRIPT_DIR=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )

cd "$SCRIPT_DIR/.."

if [ ! -f "build/info.json" ]
then
    echo "Error: Please run ./configure.rb first"
    exit 1
fi

BUILD_DIR="$(cat build/info.json | jq -r '.build_dir')"
TARGET_DIR="$(cat build/info.json | jq -r '.rust_target_dir')"
COVERAGE_DIR="$(cat build/info.json | jq -r '.coverage_dir')"

printf "BUILD_DIR: %s\nTARGET_DIR: %s\nCOVERAGE_DIR: %s\n" "$BUILD_DIR" "$TARGET_DIR" "$COVERAGE_DIR"

export RUSTFLAGS='-C instrument-coverage'
export LLVM_PROFILE_FILE="$COVERAGE_DIR/cov-%p-%16m.profraw"
export AFL_FUZZER_LOOPCOUNT=20 # without it no coverage will be written!

LLVM_TOOLS_BIN="$(rustc --print target-libdir)/../bin"

PROFILE_FILES=""

FUZZ_HELP_SHOWN=false

function help_with_fuzz() {
    if [ "$FUZZ_HELP_SHOWN" = true ]; then
        return
    fi
    FUZZ_HELP_SHOWN=true
    echo "To run fuzzing you may need to run:"
    echo '=== commands ==='
    echo 'echo core | sudo tee /proc/sys/kernel/core_pattern'
    echo 'echo performance | sudo tee /sys/devices/system/cpu/cpu*/cpufreq/scaling_governor'
    echo 'echo 0 | sudo tee /proc/sys/kernel/yama/ptrace_scope'
    echo '=== end ==='
}

function echo_and_run() {
    echo ">>> cd '$(pwd)' && " "$@"
    "$@"
}

for dir in $(git ls-files | grep -P 'Cargo\.toml' | sort)
do
    dir="$(dirname -- $dir)"
    pushd "$dir" 2> /dev/null > /dev/null

    if test -d "tests"
    then
        if !(echo "$dir/tests" | grep -P "$FILTER" > /dev/null)
        then
            echo "warn: skip $dir/tests"
        else
            echo "=== testing $dir ==="
            if [[ "$dir" == "modules/implementation" ]]
            then
                EXTRA_ARGS="--features vendored-lua"
            else
                EXTRA_ARGS=""
            fi
            echo_and_run cargo test --target-dir "$TARGET_DIR" --tests $EXTRA_ARGS

            BUILT_FILE="$(cargo test --target-dir "$TARGET_DIR" --tests $EXTRA_ARGS --no-run --message-format=json | jq -r 'select(.reason == "compiler-artifact" and .target.kind[] == "bin") | .executable')"
            PROFILE_FILES="$PROFILE_FILES --object $BUILT_FILE"
        fi
    fi

    if test -d "fuzz"
    then
        for name in $(ls fuzz/*.rs)
        do
            name="$(basename "$name" ".rs")"

            if !(echo "$dir/fuzz/$name" | grep -P "$FILTER" > /dev/null)
            then
                echo "warn: skip $dir/fuzz/$name"
            else
                help_with_fuzz

                echo "=== fuzzing $dir/fuzz/$name ==="

                cargo afl build \
                    --target-dir "$TARGET_DIR" \
                    --example "fuzz-$name"

                mkdir -p "$BUILD_DIR/genvm-testdata-out/fuzz/" || true

                echo_and_run cargo afl fuzz \
                    -c - \
                    -i "./fuzz/inputs-$name" \
                    -o "$BUILD_DIR/genvm-testdata-out/fuzz/$name" \
                    -V "$FUZZ_TIMEOUT" \
                    "$TARGET_DIR/debug/examples/fuzz-$name"

                PROFILE_FILES="$PROFILE_FILES --object $TARGET_DIR/debug/examples/fuzz-$name"
            fi
        done
    fi

    popd  2> /dev/null > /dev/null
done

if !(echo "tests" | grep -P "$FILTER" > /dev/null)
then
    echo "warn: skip tests"
else
    PROFILE_FILES="$PROFILE_FILES --object $BUILD_DIR/out/bin/genvm --object $BUILD_DIR/out/bin/genvm-modules"

    ./build/out/bin/genvm-modules llm --die-with-parent &
    LLM_JOB_ID=$!

    ./build/out/bin/genvm-modules web --die-with-parent &
    WEB_JOB_ID=$!

    if [ "$PRECOMPILE" == "true" ]
    then
        ./build/out/bin/genvm precompile
    fi

    sleep 5

    if !(kill -0 $LLM_JOB_ID)
    then
        echo "err: llm module died"
        exit 1
    fi

    if !(kill -0 $WEB_JOB_ID)
    then
        echo "err: web module died"
        exit 1
    fi

    ./tests/runner/run.py --ci --gen-vm $(readlink -f ./build/out/executor/vTEST/bin/genvm)

    kill -TERM $LLM_JOB_ID $WEB_JOB_ID

    wait $LLM_JOB_ID
    wait $WEB_JOB_ID
fi

find "$COVERAGE_DIR" -name '*.profraw' > "$COVERAGE_DIR/files-list"

"$LLVM_TOOLS_BIN/llvm-profdata" merge \
    -sparse \
    -f "$COVERAGE_DIR/files-list" \
    -o "$COVERAGE_DIR/merged.profdata"

echo "$LLVM_TOOLS_BIN/llvm-cov" report \
    -format=text \
    -instr-profile="$COVERAGE_DIR/merged.profdata" \
    --ignore-filename-regex='(^|/)(\.cargo|\.rustup|third-party)/|cranelift|target-lexicon' \
    $PROFILE_FILES

"$LLVM_TOOLS_BIN/llvm-cov" report \
    -format=text \
    -instr-profile="$COVERAGE_DIR/merged.profdata" \
    --ignore-filename-regex='(^|/)(\.cargo|\.rustup|third-party)/|cranelift|target-lexicon' \
    $PROFILE_FILES | tee "$COVERAGE_DIR/report.txt"

#!/usr/bin/env bash

set -e

cd "$(dirname ${0})"

if ! command -v git &> /dev/null; then
    echo "Git is not installed!"
    exit 2
fi

isolated=false
autoinstall=false
recompile_build_script=false

for argx in "$@"; do
    case ${argx} in
        --recompile-build-script) recompile_build_script=true ;;
        --isolate) isolated=true; autoinstall=true ;;
        --autoinstall) autoinstall=true ;;
        *) break ;;
    esac
done

_not_found() {
    while [ -v 1 ]; do
        if ! command -v ${1} &> /dev/null; then
            echo "'$1' was not found."
            return 0
        fi
        shift
    done
    return 1
}

_confirm() {
    if [ -v 1 ]; then
        echo "$*"
    fi
    printf "[yn] > "
    read res
    [ "$res" == "y" ] || [ "$res" == "Y" ] || false
}

if [ "$isolated" == "true" ] || _not_found cargo rustc; then
    export PATH="$(realpath .)/.cache/tools/rust/cargo/bin/:$PATH"
    if ! PATH="$(realpath .)/.cache/tools/rust/cargo/bin/" command -v cargo &> /dev/null; then
        if [ "$autoinstall" == "true" ] || _confirm "Create a local rust install?"; then
            mkdir -p .cache/tools/rust
            export RUSTUP_HOME="$(realpath .cache/tools/rust)"/rustup
            export CARGO_HOME="$(realpath .cache/tools/rust)"/cargo
            curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- \
                --no-modify-path \
                --default-toolchain nightly \
                --profile default \
                -y
        else
            echo ""
            echo "Follow instructions at 'https://rustup.rs/' and restart build."
            exit 1
        fi
    fi
fi

if [ ! -f Cargo.toml ]; then
    cat > Cargo.toml <<EOF
[workspace]
resolver = "3"
members = ["buildscript"]
EOF
fi

buildscript_sum="$(stat buildscript/**/* | sha256sum | cut -f1 -d\ )"
buildscript_sum_old="$(cat .cache/tools/buildscript/version_hash 2>/dev/null || true)"
if ! [ "$buildscript_sum" == "$buildscript_sum_old" ] || [ "$recompile_build_script" == "true" ]; then
    mkdir -p .cache/tools/buildscript
    cargo build --package buildscript
    printf "%s" "$buildscript_sum" > .cache/tools/buildscript/version_hash
fi

exec .cache/rust/debug/buildscript "$@"

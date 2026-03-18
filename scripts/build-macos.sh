#!/usr/bin/env bash

PKGNAME="please"
PKGVERSION="$(cargo metadata --format-version 1 | jq -r ".packages[] | select(.name==\"$PKGNAME\") | .version")"

# Use CARGO_BUILD_TARGET if set (for cross-compilation), otherwise detect host architecture
if [ -n "$CARGO_BUILD_TARGET" ]; then
  ARCH=$(echo "$CARGO_BUILD_TARGET" | cut -d'-' -f1)
else
  ARCH="$(uname -m)"
fi

rm -rf "./final/$PKGNAME-$PKGVERSION-macOS-$ARCH"

rustup component add rust-src --toolchain nightly

BUILD_CMD="RUSTFLAGS=\"-Zlocation-detail=none -Zfmt-debug=none\" cargo +nightly build -Z build-std=std,panic_abort -Z build-std-features=\"optimize_for_size\" --release"
if [ -n "$CARGO_BUILD_TARGET" ]; then
  BUILD_CMD="$BUILD_CMD --target $CARGO_BUILD_TARGET"
fi
eval "$BUILD_CMD"

mkdir -p "./final/$PKGNAME-$PKGVERSION-macOS-$ARCH"

# Binary location depends on whether we cross-compiled
if [ -n "$CARGO_BUILD_TARGET" ]; then
  BINARY_PATH="./target/$CARGO_BUILD_TARGET/release/$PKGNAME"
else
  BINARY_PATH="./target/release/$PKGNAME"
fi

mv "$BINARY_PATH" "./final/$PKGNAME-$PKGVERSION-macOS-$ARCH/$PKGNAME"
cd "./final/$PKGNAME-$PKGVERSION-macOS-$ARCH" || (echo "Critical error: Failed to change directory to './final/$PKGNAME-$PKGVERSION-macOS-$ARCH'" && exit 1)
chmod +x "$PKGNAME"
strip "$PKGNAME"

cd "../.."
#!/usr/bin/env bash

PKGNAME="please"
PKGVERSION="$(cargo metadata --format-version 1 | jq -r ".packages[] | select(.name==\"$PKGNAME\") | .version")"
ARCH="$(uname -m)"

rm -rf "./final/$PKGNAME-$PKGVERSION-Linux-$ARCH"

rustup component add rust-src --toolchain nightly

RUSTFLAGS="-Zlocation-detail=none -Zfmt-debug=none" cargo +nightly build -Z build-std=std,panic_abort -Z build-std-features="optimize_for_size" --release

mkdir -p "./final/$PKGNAME-$PKGVERSION-Linux-$ARCH"
mv "./target/release/$PKGNAME" "./final/$PKGNAME-$PKGVERSION-Linux-$ARCH/$PKGNAME"
cd "./final/$PKGNAME-$PKGVERSION-Linux-$ARCH" || (echo "Critical error: Failed to change directory to './final/$PKGNAME-$PKGVERSION-Linux-$ARCH'" && exit 1)
chmod +x "$PKGNAME"
upx --ultra-brute "$PKGNAME"
upx -t "$PKGNAME"

cd "../.."
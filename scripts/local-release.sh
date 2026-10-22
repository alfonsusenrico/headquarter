#!/bin/sh
# local-release.sh: packs this machine's release build into archives laid out as the
# release workflow publishes them, in target/local-release, for
#   HQ_INSTALL_FROM=target/local-release sh scripts/install.sh
# The app keeps runtime shaders (no Xcode needed); real releases precompile them.
set -eu
repo=$(cd "$(dirname "$0")/.." && pwd)
out=$repo/target/local-release
case $(uname -m) in arm64 | aarch64) arch=aarch64 ;; *) arch=x86_64 ;; esac
case $(uname -s) in Darwin) target=$arch-apple-darwin ;; *) target=$arch-unknown-linux-gnu ;; esac

cd "$repo"
cargo build --release -p hq-cli -p hq-app
rm -rf "$out"
mkdir -p "$out/hq-cli-$target" "$out/hq-app-$target"
cp target/release/hq target/release/headquarter README.md LICENSE-MIT LICENSE-APACHE \
  "$out/hq-cli-$target/"
cp target/release/headquarter-app assets/icon/Headquarter.icns crates/hq-app/themes/NOTICE.md \
  README.md LICENSE-MIT LICENSE-APACHE "$out/hq-app-$target/"
cp -R assets/icon/png "$out/hq-app-$target/png"
for package in hq-cli hq-app; do
  archive=$package-$target.tar.gz
  tar -czf "$out/$archive" -C "$out" "$package-$target"
  rm -rf "${out:?}/$package-$target"
  (cd "$out" && shasum -a 256 "$archive" >"$archive.sha256")
done
cp scripts/install.sh "$out/"
echo "$out"

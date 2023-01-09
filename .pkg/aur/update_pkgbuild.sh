#! /bin/bash

set -e

red="\e[38;5;1m"
green="\e[38;5;2m"
bold="\e[1m"
reset="\e[0m"

if [ -z "$PKGBUILD" ]; then
  >&2 printf "  %b%b✕%b PKGBUILD not set\n" "$red" "$bold" "$reset"
  exit 1
fi

if [ -z "$PKG_NAME" ]; then
  >&2 printf "  %b%b✕%b PKG_NAME not set\n" "$red" "$bold" "$reset"
  exit 1
fi

if [ -z "$RELEASE_TAG" ]; then
  >&2 printf "  %b%b✕%b RELEASE_TAG not set\n" "$red" "$bold" "$reset"
  exit 1
fi

if ! [ -a "$PKGBUILD" ]; then
  >&2 printf "  %b%b✕%b no such file $PKGBUILD\n" "$red" "$bold" "$reset"
  exit 1
fi

if ! [[ "$RELEASE_TAG" =~ ^v.*? ]]; then
  >&2 printf "  %b%b✕%b invalid tag $RELEASE_TAG\n" "$red" "$bold" "$reset"
  exit 1
fi

pkgver="${RELEASE_TAG#v}"

if ! [ -a "$PKG_NAME-v$pkgver".tar.gz ]; then
  >&2 printf "  %b%b✕%b no such file $PKG_NAME-v$pkgver.tar.gz\n" "$red" "$bold" "$reset"
  exit 1
fi

# bump package version
sed -i "s/pkgver=.*/pkgver=$pkgver/" "$PKGBUILD"
printf "  %b%b✓%b bump version to $RELEASE_TAG\n" "$green" "$bold" "$reset"

# generate new checksum
sum=$(set -o pipefail && sha256sum "$PKG_NAME-v$pkgver".tar.gz | awk '{print $1}')
sed -i "s/sha256sums=('.*')/sha256sums=('$sum')/" "$PKGBUILD"
printf "  %b%b✓%b generated checksum $sum\n" "$green" "$bold" "$reset"

exit 0

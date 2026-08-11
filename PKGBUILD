# Maintainer: FBII Contributors <fbii@example.com>
pkgname=fbii
pkgver=0.1.0
pkgrel=1
pkgdesc="A terminal e-book reader for FB2, FB2-in-ZIP and EPUB with vim-like controls built in Rust"
arch=('x86_64' 'aarch64')
url="https://github.com/zsh-ncursed/fbii"
license=('MIT')
depends=('gcc-libs' 'sqlite')
makedepends=('cargo')
source=("$pkgname-$pkgver.tar.gz::$url/archive/v$pkgver.tar.gz")
sha256sums=('SKIP')

prepare() {
  cd "$pkgname-$pkgver"
  cargo fetch --locked --target "$CARCH-unknown-linux-gnu"
}

build() {
  cd "$pkgname-$pkgver"
  export CARGO_TARGET_DIR=target
  cargo build --frozen --release --all-targets
}

check() {
  cd "$pkgname-$pkgver"
  cargo test --frozen
}

package() {
  cd "$pkgname-$pkgver"
  install -Dm755 "target/release/$pkgname" "$pkgdir/usr/bin/$pkgname"
  install -Dm644 LICENSE "$pkgdir/usr/share/licenses/$pkgname/LICENSE"
  install -Dm644 README.md "$pkgdir/usr/share/doc/$pkgname/README.md"
}

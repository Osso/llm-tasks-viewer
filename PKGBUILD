# Maintainer: Alessio Deiana <adeiana@gmail.com>
pkgname=llm-tasks-viewer
pkgver=0.1.0
pkgrel=1
pkgdesc="Dioxus desktop app for viewing llm-tasks in real-time"
arch=('x86_64')
license=('MIT')
makedepends=('cargo')
source=()

build() {
    cd "$startdir"
    cargo build --release --locked
}

package() {
    cd "$startdir"
    install -d "$pkgdir/usr/bin"
    install -m755 "target/release/llm-tasks-viewer" "$pkgdir/usr/bin/llm-tasks-viewer"
}

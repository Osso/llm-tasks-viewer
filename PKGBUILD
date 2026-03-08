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
    install -Dm755 "target/release/llm-tasks-viewer" "$pkgdir/usr/bin/llm-tasks-viewer"
    install -Dm644 "assets/llm-tasks-viewer.desktop" "$pkgdir/usr/share/applications/llm-tasks-viewer.desktop"
    install -Dm644 "assets/llm-tasks-viewer.svg" "$pkgdir/usr/share/icons/hicolor/scalable/apps/llm-tasks-viewer.svg"
}

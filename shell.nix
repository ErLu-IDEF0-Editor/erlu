{ pkgs ? import <nixpkgs> {} }:
let runtimeLibraries = with pkgs; [ libGL libxkbcommon wayland libx11 libxcursor libxrandr libxi ];
in pkgs.mkShell {
  packages = with pkgs; [ cargo rustc rustfmt clippy pkg-config lld trunk wasm-bindgen-cli wasm-tools nodejs python3 ] ++ runtimeLibraries;
  LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath runtimeLibraries;
  shellHook = ''
    export NO_COLOR=true
    # Never share rmeta/build scripts with rustup or another Nix Rust compiler.
    export RUSTC=${pkgs.rustc}/bin/rustc
    export RUSTDOC=${pkgs.rustc}/bin/rustdoc
    export PATH=${pkgs.cargo}/bin:$PATH
    export CARGO_TARGET_DIR="$PWD/target/nix-${builtins.baseNameOf (toString pkgs.rustc)}"
    echo "erlu: cargo run -p erlu; npm --prefix web ci; npm --prefix web run plugins; trunk serve"
    echo "The Rust compiler must include wasm32-unknown-unknown (rustup target add on non-Nix toolchains)."
  '';
}

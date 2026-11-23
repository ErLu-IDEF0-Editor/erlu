{ pkgs ? import <nixpkgs> {} }:
let
  inherit (pkgs) lib;
  mkPlugin = id: artifact: pkgs.rustPlatform.buildRustPackage {
    pname = "erlu-${id}";
    version = "0.1.0";
    src = lib.cleanSourceWith {
      src = ./plugins + "/${id}";
      filter = path: type: lib.cleanSourceFilter path type && !(builtins.elem (builtins.baseNameOf path) [ "target" "dist" ]);
    };
    cargoLock.lockFile = ./plugins + "/${id}/Cargo.lock";
    nativeBuildInputs = [ pkgs.wasm-tools pkgs.lld ];
    CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUSTFLAGS = "-C link-arg=--max-memory=33554432";
    buildPhase = ''
      runHook preBuild
      cargo build --release --frozen --target wasm32-unknown-unknown
      runHook postBuild
    '';
    doCheck = false;
    installPhase = ''
      mkdir -p $out/dist
      wasm-tools component new target/wasm32-unknown-unknown/release/${artifact}.wasm -o $out/dist/${id}.wasm
      cp plugin.json $out/plugin.json
    '';
  };
  colors = mkPlugin "colors" "erlu_colors";
  responsibilityMap = mkPlugin "responsibility_map" "erlu_responsibility_map";
  semanticDictionary = mkPlugin "semantic_dictionary" "erlu_semantic_dictionary";
  runtimeLibraries = with pkgs; [ libGL libxkbcommon wayland libx11 libxcursor libxrandr libxi ];
in pkgs.rustPlatform.buildRustPackage {
  pname = "erlu";
  version = "0.1.0";
  src = lib.cleanSourceWith {
    src = ./.;
    filter = path: type:
      lib.cleanSourceFilter path type
      && !(builtins.elem (builtins.baseNameOf path) [ "target" "dist" "tmp" "output" "__pycache__" "node_modules" ".tools" "test-results" "playwright-report" ]);
  };
  cargoLock.lockFile = ./Cargo.lock;
  nativeBuildInputs = with pkgs; [ pkg-config makeWrapper ];
  buildInputs = runtimeLibraries;
  cargoBuildFlags = [ "-p" "erlu" ];
  cargoTestFlags = [ "-p" "erlu-core" ];
  postInstall = ''
    mkdir -p $out/bin/plugins/colors
    cp -r ${colors}/* $out/bin/plugins/colors/
    mkdir -p $out/bin/plugins/responsibility_map
    cp -r ${responsibilityMap}/* $out/bin/plugins/responsibility_map/
    mkdir -p $out/bin/plugins/semantic_dictionary
    cp -r ${semanticDictionary}/* $out/bin/plugins/semantic_dictionary/
    wrapProgram $out/bin/erlu --prefix LD_LIBRARY_PATH : ${lib.makeLibraryPath runtimeLibraries}
    install -Dm644 packaging/linux/erlu-idef0-editor.desktop $out/share/applications/erlu.desktop
    install -Dm644 packaging/linux/idef0.xml $out/share/mime/packages/erlu.xml
    install -Dm644 assets/icons/erlu.svg $out/share/icons/hicolor/scalable/apps/erlu.svg
  '';
  meta = with lib; {
    description = "IDEF0 editor with egui, MCP and portable WASM plugins";
    license = licenses.lgpl3Plus;
    mainProgram = "erlu";
    platforms = platforms.linux ++ platforms.darwin;
  };
}

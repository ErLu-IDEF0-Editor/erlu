{ pkgs ? import <nixpkgs> {}
, buildColorsPlugin ? true
}:

let
  inherit (pkgs) lib stdenv cmake pkg-config;
  qt = pkgs.qt6;
in
stdenv.mkDerivation rec {
  pname = "idef0_editor";
  version = "0.1.0";

  src = lib.cleanSourceWith {
    src = ./.;
    filter = path: type:
      (lib.cleanSourceFilter path type)
      && (builtins.baseNameOf path != "build");
  };

  nativeBuildInputs = [
    cmake
    pkg-config
    qt.wrapQtAppsHook
    qt.qttools
  ];

  buildInputs = [
    qt.qtbase
  ];

  configurePhase = ''
    cmake -S . -B build -DCMAKE_BUILD_TYPE=Release \
      -DCMAKE_INSTALL_PREFIX=$out \
      -DBUILD_COLORS_PLUGIN=${if buildColorsPlugin then "ON" else "OFF"}
  '';

  buildPhase = ''
    cmake --build build
    find . -path "*/translations/*.ts" -print0 | while IFS= read -r -d "" f; do
      out_path=$(printf '%s\n' "$f" | sed 's/\.ts$/.qm/')
      lrelease "$f" -qm "$out_path"
    done
  '';

  installPhase = ''
    runHook preInstall
    cmake --install build --prefix $out
    # app translations
    if [ -d translations ]; then
      mkdir -p $out/share/idef0/translations
      find translations -maxdepth 1 -type f -name '*.qm' -print0 | xargs -0 -r cp -t $out/share/idef0/translations
    fi
    # plugin translations
    find src/plugins -path "*/translations/*.qm" -print0 | while IFS= read -r -d "" f; do
      rel="''${f#src/plugins/}"          # e.g., color/translations/colors_en.qm
      plugdir="''${rel%%/translations/*}" # e.g., color
      dest="$out/plugins/$plugdir/translations"
      mkdir -p "$dest"
      cp "$f" "$dest/"
    done
    runHook postInstall
  '';

  qtWrapperArgs = [
    "--set" "QT_LOGGING_RULES" "qt.qpa.wayland.textinput=false"
  ];

  meta = with lib; {
    description = "IDEF0 diagram editor built with Qt 6 Widgets";
    license = licenses.lgpl3Plus;
    mainProgram = "idef0_editor";
    platforms = platforms.linux;
  };
}

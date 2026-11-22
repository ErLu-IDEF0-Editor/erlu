{ pkgs ? import <nixpkgs> {} }:
# Nix-built ELF files need their bundled loader as well as relative library paths.
let
  inherit (pkgs) lib;
  erlu = import ./default.nix { inherit pkgs; };
  closure = pkgs.closureInfo { rootPaths = [ erlu ]; };
  source = pkgs.fetchurl {
    url = "https://github.com/AppImage/appimagetool/releases/download/continuous/appimagetool-x86_64.AppImage";
    hash = "sha256-6MEWnEr2bDGE1+4RBxPUvgS+/mcfo1jiwVke6EBVOwM=";
  };
  extracted = pkgs.appimageTools.extractType2 { pname = "appimagetool"; version = "pinned-e8c1169c"; src = source; };
  tool = pkgs.stdenv.mkDerivation {
    pname = "erlu-appimagetool";
    version = "pinned-e8c1169c";
    src = extracted;
    nativeBuildInputs = [ pkgs.autoPatchelfHook ];
    buildInputs = with pkgs; [ glib zlib libarchive fuse openssl ];
    installPhase = ''mkdir -p $out; cp -r . $out/'';
    dontStrip = true;
  };
in pkgs.runCommand "erlu-${erlu.version}-appimage" {
  nativeBuildInputs = with pkgs; [ patchelf binutils squashfsTools file findutils coreutils ];
  meta.platforms = [ "x86_64-linux" ];
} ''
  set -euo pipefail
  appdir="$PWD/erlu.AppDir"
  mkdir -p "$appdir/usr/bin" "$appdir/usr/lib"
  cp ${erlu}/bin/.erlu-wrapped "$appdir/usr/bin/erlu"
  cp -r ${erlu}/bin/plugins "$appdir/usr/bin/"
  while IFS= read -r store_path; do
    for libdir in "$store_path/lib" "$store_path/lib64"; do
      if [ -d "$libdir" ]; then
        while IFS= read -r shared_object; do
          cp -L "$shared_object" "$appdir/usr/lib/$(basename "$shared_object")"
        done < <(find -L "$libdir" -maxdepth 1 -type f -name '*.so*')
      fi
    done
  done < ${closure}/store-paths
  chmod -R u+w "$appdir"
  patchelf --set-rpath '$ORIGIN/../lib' "$appdir/usr/bin/erlu"
  for shared_object in "$appdir"/usr/lib/*; do
    if patchelf --print-rpath "$shared_object" >/dev/null 2>&1; then
      patchelf --set-rpath '$ORIGIN' "$shared_object"
    fi
  done
  cp ${./packaging/linux/erlu-idef0-editor.desktop} "$appdir/erlu.desktop"
  cp ${./assets/icons/erlu.svg} "$appdir/erlu.svg"
  cat > "$appdir/AppRun" <<'RUN'
#!/bin/sh
set -eu
app_root=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
export ERLU_PLUGIN_DIR="$app_root/usr/bin/plugins"
export ERLU_EXECUTABLE="''${APPIMAGE:-$app_root/AppRun}"
export LD_LIBRARY_PATH="$app_root/usr/lib:/run/opengl-driver/lib:/usr/lib/x86_64-linux-gnu''${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
exec "$app_root/usr/lib/ld-linux-x86-64.so.2" --library-path "$LD_LIBRARY_PATH" "$app_root/usr/bin/erlu" "$@"
RUN
  chmod +x "$appdir/AppRun"
  cp ${source} tool.AppImage
  chmod +x tool.AppImage
  offset=$(./tool.AppImage --appimage-offset)
  head -c "$offset" tool.AppImage > runtime
  mkdir -p "$out"
  ARCH=x86_64 ${tool}/usr/bin/appimagetool --runtime-file "$PWD/runtime" "$appdir" "$out/erlu-${erlu.version}-x86_64.AppImage"
''

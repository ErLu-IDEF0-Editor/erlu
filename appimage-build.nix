{ pkgs ? import <nixpkgs> {}
, buildColorsPlugin ? true
}:

# Builds a distributable AppImage from the local sources (see default.nix).
# Uses linuxdeployqt + appimagetool AppImages to bundle Qt runtime bits and
# generate the final .AppImage artifact.
let
  inherit (pkgs) lib;

  # Reuse the main package but skip the nixpkgs Qt wrapper so we can lay down
  # the raw binaries and their closure into the AppDir.
  erlu = (import ./default.nix {
    inherit pkgs buildColorsPlugin;
  }).overrideAttrs (_: {
    dontWrapQtApps = true;
  });

  appimagetoolSrc = pkgs.fetchurl {
    url = "https://github.com/AppImage/appimagetool/releases/download/continuous/appimagetool-x86_64.AppImage";
    sha256 = "sha256-ptceK2zWb46NFsN60WRliYXgz1/KqVDJCkgokMudE+A=";
  };
in
  pkgs.runCommand "erlu-idef0-editor-appimage"
  {
    nativeBuildInputs = [
      pkgs.squashfsTools
      pkgs.patchelf
      pkgs.binutils
      pkgs.findutils
      pkgs.coreutils
      pkgs.file
      pkgs.imagemagick
      pkgs.nix
    ];
  } ''
    set -euo pipefail
    export HOME=$PWD/home
    mkdir -p "$HOME"

    # Extract appimagetool AppImage without running it
    extract_appimage() {
      local src="$1"
      local dest="$2"
      mkdir -p "$dest"
      local offset
      offset=$(LC_ALL=C readelf -h "$src" | awk 'NR==13{e_shoff=$5} NR==18{e_shentsize=$5} NR==19{e_shnum=$5} END{print e_shoff+e_shentsize*e_shnum}')
      unsquashfs -q -d "$dest" -o "$offset" "$src"
      chmod -R u+w "$dest"
    }

    extract_appimage ${appimagetoolSrc} appimagetool

    appdir=$PWD/AppDir
    mkdir -p "$appdir"/usr/{bin,share/applications,share/icons/hicolor/scalable/apps,share/mime/packages}

    # Core binary and runtime data
    cp ${erlu}/bin/erlu_idef0_editor "$appdir/usr/bin/erlu_idef0_editor"
    chmod +x "$appdir/usr/bin/erlu_idef0_editor"

    if [ -d ${erlu}/share ]; then
      cp -r ${erlu}/share/* "$appdir/usr/share/"
    fi

    if [ -d ${erlu}/plugins ]; then
      cp -r ${erlu}/plugins "$appdir/usr/"
    fi

    # Ensure we can amend the copied tree (Nix store files are read-only)
    chmod -R u+w "$appdir"

    # Copy the full dependency closure into /nix/store inside the AppDir so
    # the existing RPATHs and interpreter paths keep working at runtime.
    mkdir -p "$appdir/nix/store"
    for p in $(nix-store -qR ${erlu}); do
      cp -a "$p" "$appdir/nix/store/"
    done

    # Desktop integration bits
    cp ${./packaging/linux/erlu-idef0-editor.desktop} "$appdir/usr/share/applications/erlu-idef0-editor.desktop"
    cp ${./assets/icons/erlu.svg} "$appdir/usr/share/icons/hicolor/scalable/apps/erlu.svg"
    cp ${./packaging/linux/idef0.xml} "$appdir/usr/share/mime/packages/idef0.xml"

    # linuxdeployqt expects the .desktop and icon at the AppDir root too
    cp "$appdir/usr/share/applications/erlu-idef0-editor.desktop" "$appdir/"
    cp "$appdir/usr/share/icons/hicolor/scalable/apps/erlu.svg" "$appdir/erlu.svg"

    # Minimal AppRun launcher
    cat > "$appdir/AppRun" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
HERE="$(dirname "$(readlink -f "$0")")"
export PATH="$HERE/usr/bin:$PATH"
exec "$HERE/usr/bin/erlu_idef0_editor" "$@"
EOF
    chmod +x "$appdir/AppRun"

    # Derive the type-2 runtime from the appimagetool AppImage (avoids network fetch)
    runtime=$PWD/runtime
    runtime_offset=$(LC_ALL=C readelf -h ${appimagetoolSrc} | awk 'NR==13{e_shoff=$5} NR==18{e_shentsize=$5} NR==19{e_shnum=$5} END{print e_shoff+e_shentsize*e_shnum}')
    head -c "$runtime_offset" ${appimagetoolSrc} > "$runtime"
    chmod +x "$runtime"

    # Wrapper to force appimagetool to use the local runtime (and log its use)
    cat > appimagetool-wrapper <<EOF
#!${pkgs.coreutils}/bin/env bash
set -euxo pipefail
echo "appimagetool-wrapper: runtime=$runtime pwd=\$PWD args=\$@" >&2
ls -l "$PWD/appimagetool/usr/bin" >&2 || true
exec "$PWD/appimagetool/usr/bin/appimagetool" --runtime-file "$runtime" "\$@"
EOF
    chmod +x appimagetool-wrapper

    export PATH="$PWD/appimagetool/usr/bin:${pkgs.file}/bin:$PATH"

    # Build the AppImage directly with appimagetool
    ./appimagetool-wrapper "$appdir"

    mkdir -p "$out"
    mv ./*.AppImage "$out/erlu-idef0-editor-${erlu.version}.AppImage"
  ''

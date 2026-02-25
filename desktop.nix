{ pkgs ? import <nixpkgs> {} }:

# Desktop-focused Nix entrypoint for NixOS distributions.
# Currently this reuses default.nix, which already installs:
# - binary
# - translations
# - .desktop launcher
# - MIME package (application/x-idef0+json for *.idef0)
import ./default.nix { inherit pkgs; }

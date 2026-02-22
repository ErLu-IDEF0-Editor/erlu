{ pkgs ? import <nixpkgs> {} }:

pkgs.mkShell {

  buildInputs = (with pkgs.qt6; [
qt3d
qtdatavis3d
qthttpserver
qtquick3d
qtscxml
qtspeech
qtwayland
qt5compat
qtdeclarative
qtimageformats
qtmqtt
qtquick3dphysics
qtsensors
qtsvg
qtwebchannel
qtbase
qtdoc
qtlanguageserver
qtmultimedia
qtquickeffectmaker
qtserialbus
qttools
qtwebengine
qtcharts
qtgraphs
qtlocation
qtnetworkauth
qtquicktimeline
qtserialport
qttranslations
qtwebsockets
qmake
qtconnectivity
qtgrpc
qtlottie
qtpositioning
qtremoteobjects
qtshadertools
qtvirtualkeyboard
qtwebview
]) ++ (with pkgs; [
    cmake
    qtcreator
  ]);

}

let
  pkgs = import <nixpkgs> {
    config = {
      android_sdk.accept_license = true;
      allowUnfree = true;
    };
  };

  androidPkgs = pkgs.androidenv.composeAndroidPackages {
    platformVersions = [
      "34"
      "35"
      "36"
    ];
    buildToolsVersions = [
      "35.0.0"
    ];
    includeNDK = "if-supported";
    ndkVersions = [ "28.2.13676358" ];
    includeCmake = true;
    cmakeVersions = [ "3.22.1" ];
    includeEmulator = true;
    includeSystemImages = true;
  };

  emulator = pkgs.androidenv.emulateApp {
    name = "flutter_emu";
    platformVersion = "35";
    abiVersion = "x86";
    systemImageType = "google_apis_playstore";
    deviceName = "pixel";
  };
in
pkgs.mkShell {
  ANDROID_SDK_ROOT = "${androidPkgs.androidsdk}/libexec/android-sdk";
  ANDROID_HOME = "${androidPkgs.androidsdk}/libexec/android-sdk";
  JAVA_HOME = pkgs.jdk17.home;

  buildInputs = with pkgs; [
    fvm
    just
    jdk17
    androidPkgs.androidsdk
    emulator
    unzip
  ];

  shellHook = ''
    echo "FVM:     $(fvm --version 2>/dev/null | head -n 1)"
    echo "Flutter: $(fvm flutter --version | head -n 1)"
    echo "Java:    $(java -version 2>&1 | head -n 1)"
    echo "SDK:     $ANDROID_SDK_ROOT"
    echo
    echo "Setup emulator: (just create_emulator)"
    echo "  avdmanager create avd -n flutter_emulator -k 'system-images;android-35;google_apis_playstore;x86_64' -d pixel"
    echo
    echo "Build develop branch:"
    echo "  fvm install"
    echo "  fvm flutter pub get"
    echo "  fvm dart run build_runner build --delete-conflicting-outputs"
    echo "  fvm flutter build apk --flavor develop --debug"
    echo
    echo "Run locally: (just start_emulator && just dev)"
    echo "  fvm flutter emulators --launch flutter_emulator"
    echo "  fvm flutter run --flavor develop -d flutter_emulator"

    if [ ! -f .env ]; then
      echo
      echo "Note: .env is missing in the current directory; envied code generation will fail until it exists."
    fi
  '';
}

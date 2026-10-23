# Personal jidoujisho build

Vendored from https://github.com/arianneorpilla/jidoujisho at commit cc7e4925.
The upstream project is dead. This is a personal build.

This directory contains upstream `yuuna/` and its GPL-3.0 license.

## Personal Android releases

The application ID is `com.btrkeks.jidoujisho`. The launcher label is still
`jidoujisho`. Android App info distinguishes it from the old installation.
The Java package and native channel names remain `app.arianneorpilla.yuuna`.

Use Flutter 3.13.5, JDK 17, and the Android SDK. From this directory:

```sh
python3 tool/ttu_bundle/update_ttu_bundle.py --verify
flutter pub get --enforce-lockfile
flutter build apk --release --target-platform android-arm64 --no-pub
flutter build apk --release --target-platform android-x64 --no-pub
```

Each build writes `build/app/outputs/flutter-apk/app-release.apk`. Copy it before
building another architecture. Signing reads the absolute default path
`~/.local/share/keystores/jidoujisho/key.properties` under the build user's home,
or the path in `JIDOUJISHO_KEY_PROPERTIES`. Its `storeFile` must be absolute.
The properties are `storeFile`, `storePassword`, `keyAlias`, and `keyPassword`.
Release builds fail when the key is missing. They never use the debug key.

Back up the private keystore directory securely, including `release.jks` and
`key.properties`. Losing the key means future updates require uninstalling the
app and losing its local data. Keep the directory mode 700 and signing files
mode 600. Never commit signing material.

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
flutter build apk --release --split-per-abi --target-platform android-arm64 --no-pub
flutter build apk --release --split-per-abi --target-platform android-x64 --no-pub
```

The builds write `build/app/outputs/flutter-apk/app-arm64-v8a-release.apk` and
`app-x86_64-release.apk`. Always use `--split-per-abi`. Without it, plugin native
libraries for all four ABIs double the APK to about 560 MB. The version code also
drops the ABI offset (20000 instead of 20002), so Android refuses it as a
downgrade over an installed split build. Signing reads the absolute default path
`~/.local/share/keystores/jidoujisho/key.properties` under the build user's home,
or the path in `JIDOUJISHO_KEY_PROPERTIES`. Its `storeFile` must be absolute.
The properties are `storeFile`, `storePassword`, `keyAlias`, and `keyPassword`.
Release builds fail when the key is missing. They never use the debug key.

Back up the private keystore directory securely, including `release.jks` and
`key.properties`. Losing the key means future updates require uninstalling the
app and losing its local data. Keep the directory mode 700 and signing files
mode 600. Never commit signing material.

Verify saved release APKs before installation. Obtain the expected public SHA-256
certificate fingerprint from the backed-up keystore or the release report.

```sh
python3 tool/android/verify_release.py --certificate-sha256 EXPECTED_SHA256 /path/to/arm64.apk /path/to/x64.apk
```

The check rejects a different package, a debuggable APK, a different signer, or
reader asset drift. Its JSON contains the APK hashes and absolute paths.
For emulator verification, the shared harness accepts `JIDOUJISHO_PACKAGE` with
`com.btrkeks.jidoujisho` or the old test package. The daily old package is excluded.
The launcher component is
`com.btrkeks.jidoujisho/app.arianneorpilla.yuuna.MainActivity`.
Hold the shared emulator lock and always use an explicit emulator serial.

The [cutover runbook](cutover.md) records the server, laptop, and phone steps.
`~/.local/state/pstack-t3/jidoujisho-lifestats/cutover.md` is a symlink to it.
Phone commands there are future operator steps. The phone was off limits during
vendor implementation.

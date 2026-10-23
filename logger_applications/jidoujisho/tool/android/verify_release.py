#!/usr/bin/env python3
"""Check personal release identity, signing certificate, and embedded reader bytes."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import sys
import zipfile


def verify(apk, build_tools, certificate):
    badging = subprocess.check_output([str(build_tools / 'aapt2'), 'dump', 'badging', str(apk)], text=True)
    if "package: name='com.btrkeks.jidoujisho'" not in badging or 'application-debuggable' in badging:
        raise ValueError(f'{apk}: expected a non-debuggable com.btrkeks.jidoujisho APK')
    signing = subprocess.check_output([str(build_tools / 'apksigner'), 'verify', '--print-certs', str(apk)], text=True)
    certificates = re.findall(r'Signer #\d+ certificate SHA-256 digest: ([0-9a-f]+)', signing)
    if certificates != [certificate]:
        raise ValueError(f'{apk}: release certificate differs from the expected fingerprint')
    app = Path(__file__).resolve().parents[2]
    assets = app / 'assets/ttu-ebook-reader'
    files = json.loads((assets / 'BUNDLE_SOURCE.json').read_text())['files']
    with zipfile.ZipFile(apk) as archive:
        for name in [*files, 'BUNDLE_SOURCE.json']:
            actual = archive.read('assets/flutter_assets/assets/ttu-ebook-reader/' + name)
            if actual != (assets / name).read_bytes():
                raise ValueError(f'{apk}: embedded reader drift in {name}')
    return {'path': str(apk.resolve()), 'sha256': hashlib.sha256(apk.read_bytes()).hexdigest(),
            'certificateSHA256': certificate, 'runtimeAssetsMatched': len(files),
            'package': 'com.btrkeks.jidoujisho', 'debuggable': False, 'verdict': 'VERIFIED'}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--certificate-sha256', required=True)
    parser.add_argument('--build-tools', type=Path, default=Path(os.environ.get('ANDROID_HOME', str(Path.home() / 'Android/Sdk'))) / 'build-tools/35.0.0')
    parser.add_argument('apks', type=Path, nargs='+')
    args = parser.parse_args()
    certificate = args.certificate_sha256.replace(':', '').lower()
    if not re.fullmatch('[0-9a-f]{64}', certificate):
        parser.error('Certificate fingerprint must be 64 hexadecimal digits')
    print(json.dumps([verify(apk, args.build_tools, certificate) for apk in args.apks], indent=2))


if __name__ == '__main__':
    try:
        main()
    except (ValueError, OSError, subprocess.CalledProcessError) as error:
        sys.exit(str(error))

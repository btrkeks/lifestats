import contextlib
import copy
import io
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

import update_ttu_bundle as bundle


class BundleVerificationTest(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.yuuna = Path(self.temp.name)
        self.assets = self.yuuna / 'assets/ttu-ebook-reader'
        self.assets.mkdir(parents=True)
        for name in ['index.html', 'b.html', 'manage.html', 'settings.html', 'auth.html']:
            (self.assets / name).write_text('<html></html>')
        (self.assets / '_app').mkdir()
        (self.assets / '_app/version.json').write_text('{"version":"test-build"}')
        (self.assets / '_app/reader.js').write_text('".book-content";"/auth.html?ttu-init-auth=1";')
        self.lock = self.yuuna / 'bundle.lock.json'
        config = {
            'repository': 'https://example.test/fork', 'commit': 'a' * 40,
            'dirty': False, 'sourceHash': 'b' * 64,
            'buildProfile': {'env': {'VITE_SERVICE_WORKER': 'off'}},
            'toolchain': {'node': '20.11.1', 'pnpm': '9.15.9'}
        }
        self.lock.write_text(bundle.encoded(config))
        source = copy.deepcopy(config)
        source['files'] = bundle.manifest(self.assets)
        source['contentHash'] = bundle.content_hash(source['files'])
        (self.assets / 'BUNDLE_SOURCE.json').write_text(bundle.encoded(source))
        self.pubspec = self.yuuna / 'pubspec.yaml'
        self.pubspec.write_text(bundle.BEGIN + '\n' + bundle.asset_block(self.assets) + '\n' + bundle.END + '\n')
        for key, value in [('YUUNA', self.yuuna), ('ASSETS', self.assets), ('LOCK', self.lock)]:
            context = patch.object(bundle, key, value)
            context.start()
            self.addCleanup(context.stop)

    def verify(self):
        with contextlib.redirect_stdout(io.StringIO()):
            bundle.verify()

    def test_exact_bundle_passes_and_missing_pubspec_line_fails(self):
        self.verify()
        self.pubspec.write_text(self.pubspec.read_text().replace('    - assets/ttu-ebook-reader/_app/\n', ''))
        with self.assertRaisesRegex(ValueError, 'pubspec asset-directory block drift'):
            self.verify()

    def test_changed_added_and_removed_assets_fail(self):
        for change in ['modify', 'add', 'remove']:
            with self.subTest(change=change):
                file = self.assets / ('new.js' if change == 'add' else 'b.html')
                old = file.read_bytes() if file.exists() else None
                if change == 'remove':
                    file.unlink()
                else:
                    file.write_text('drift')
                with self.assertRaisesRegex(ValueError, 'Bundle content drift'):
                    self.verify()
                if old is None:
                    file.unlink()
                else:
                    file.write_bytes(old)

    def test_source_pin_drift_fails(self):
        data = json.loads(self.lock.read_text())
        data['commit'] = 'c' * 40
        self.lock.write_text(bundle.encoded(data))
        with self.assertRaisesRegex(ValueError, 'commit'):
            self.verify()

    def test_matching_dirty_lock_and_provenance_fail_verification(self):
        self.verify()
        for file in [self.lock, self.assets / 'BUNDLE_SOURCE.json']:
            data = json.loads(file.read_text())
            data['dirty'] = True
            file.write_text(bundle.encoded(data))
        with self.assertRaisesRegex(ValueError, 'clean pinned source'):
            self.verify()

    def test_host_contracts_reject_worker_auth_and_live_default(self):
        reader = self.assets / '_app/reader.js'
        initial = reader.read_text()
        for code, failure in [
            ('navigator.serviceWorker.register("/service-worker.js")', 'service worker registration'),
            ('"/auth?ttu-init-auth=1"', 'extensionless auth'),
            ('"http://localhost:3000"', 'localhost:3000'),
        ]:
            with self.subTest(failure=failure):
                reader.write_text(initial + code)
                with self.assertRaisesRegex(ValueError, failure):
                    bundle.check_output(self.assets, {'env': {'VITE_SERVICE_WORKER': 'off'}})
        reader.write_text(initial)
        (self.assets / 'b.html').unlink()
        with self.assertRaisesRegex(ValueError, 'missing physical b.html'):
            bundle.check_output(self.assets, {'env': {'VITE_SERVICE_WORKER': 'off'}})


if __name__ == '__main__':
    unittest.main()

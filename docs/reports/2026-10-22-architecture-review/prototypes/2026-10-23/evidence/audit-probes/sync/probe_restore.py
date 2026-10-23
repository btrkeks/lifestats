"""Auditor probes against an unmodified copy of sync/run.py."""
import sqlite3, sys, tempfile
from pathlib import Path
sys.path.insert(0, str(Path(__file__).parent))
import run as m

with tempfile.TemporaryDirectory() as t:
    t = Path(t)
    # P1: plain file-copy restore (no generation bump) makes version tokens reusable.
    db = m.open_server(t/'s.db')
    m.manual_add(db,'stable_client_identity','server-a','phone-1','seed',5)          # version 1
    m.backup(db, t/'s.bak')
    m.manual_add(db,'stable_client_identity','server-a','phone-1','add-2',3)         # version 2, total 8
    seen_by_phone2 = m.meta(db,'generation')+':'+m.meta(db,'version')               # phone-2 reads base "server-a:2" (total 8)
    db.close()
    with sqlite3.connect(t/'s.bak') as src, sqlite3.connect(t/'s.db') as dst: src.backup(dst)  # restore WITHOUT minting generation
    db = m.open_server(t/'s.db')
    m.manual_add(db,'stable_client_identity','server-a','laptop','other',100)        # different write, version 2 again
    r = m.manual_add(db,'stable_client_identity','server-a','phone-2','set-x',1,kind='manual_set',expected_version=seen_by_phone2)
    print('P1 plain restore: phone-2 base token', seen_by_phone2, '-> conditional set outcome', r['status'], 'against a base it never saw; total', m.total(db))
    snap_before = m.snapshot(db,('2026-10-23','2026-10-23'),'2026-10-23','database_version_only')
    db.close()

    # P2: operation-id reuse after a client-side restore silently drops a new write.
    db = m.open_server(t/'c.db')
    a = m.manual_add(db,'stable_client_identity','server-a','phone-1','operation-1',5)
    b = m.manual_add(db,'stable_client_identity','server-a','phone-1','operation-1',5)  # restored phone reuses counter-based id for a NEW add
    print('P2 reused op id for new identical add:', a['status'], b['status'], 'total', m.total(db), '(user intended 10)')
    db.close()

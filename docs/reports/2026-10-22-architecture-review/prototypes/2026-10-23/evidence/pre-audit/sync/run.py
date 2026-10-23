#!/usr/bin/env python3
"""Throwaway SQLite experiments; results describe this model, not deployed Lifestats."""
import argparse
import hashlib
from http.server import BaseHTTPRequestHandler, HTTPServer
import json
from pathlib import Path
import sqlite3
import tempfile
import threading
from urllib.error import HTTPError
from urllib.parse import parse_qs, urlparse
from urllib.request import Request, urlopen

BASELINE = "607978998c72b1e14a95518d32fbfeafe2f7fed6"
SAFE = 2**53 - 1

def canonical(value):
    return json.dumps(value, sort_keys=True, separators=(",", ":"))

def digest(value):
    return hashlib.sha256(canonical(value).encode()).hexdigest()

def open_server(path, generation="server-a"):
    db = sqlite3.connect(path, isolation_level=None)
    db.executescript("""
        CREATE TABLE IF NOT EXISTS metadata(key TEXT PRIMARY KEY,value TEXT NOT NULL);
        CREATE TABLE IF NOT EXISTS contributors(
            metric TEXT NOT NULL, day TEXT NOT NULL, contributor TEXT NOT NULL,
            amount INTEGER NOT NULL, PRIMARY KEY(metric,day,contributor));
        CREATE TABLE IF NOT EXISTS receipts(
            namespace TEXT NOT NULL,client TEXT NOT NULL,operation_id TEXT NOT NULL,
            payload TEXT NOT NULL,outcome TEXT NOT NULL,
            PRIMARY KEY(namespace,client,operation_id));
        CREATE TABLE IF NOT EXISTS source_revisions(
            stream TEXT NOT NULL,day TEXT NOT NULL,epoch TEXT NOT NULL,
            revision INTEGER NOT NULL,payload TEXT NOT NULL,cleared INTEGER NOT NULL,
            PRIMARY KEY(stream,day));
        CREATE TABLE IF NOT EXISTS authorities(stream TEXT PRIMARY KEY,epoch TEXT NOT NULL,needs_reconciliation INTEGER NOT NULL);
    """)
    db.execute("INSERT OR IGNORE INTO metadata VALUES('version','0')")
    db.execute("INSERT OR IGNORE INTO metadata VALUES('generation',?)", (generation,))
    return db

def meta(db, key):
    return db.execute("SELECT value FROM metadata WHERE key=?", (key,)).fetchone()[0]

def bump(db):
    db.execute("UPDATE metadata SET value=CAST(value AS INTEGER)+1 WHERE key='version'")

def total(db):
    return db.execute("SELECT COALESCE(SUM(amount),0) FROM contributors").fetchone()[0]

def backup(db, path):
    with sqlite3.connect(path) as dest:
        db.backup(dest)

def restored(path, backup_path):
    with sqlite3.connect(backup_path) as source, sqlite3.connect(path) as dest:
        source.backup(dest)
    db = open_server(path)
    db.execute("UPDATE metadata SET value='server-b' WHERE key='generation'")
    return db

def manual_add(db, key_mode, request_generation, client, operation_id, amount, fail_at=None, kind='manual_add', expected_version=None):
    payload = {"kind": "manual_add", "metric": "cards", "day": "2026-10-23", "amount": amount}
    payload['kind'] = kind
    if expected_version is not None:
        payload['expected_version'] = expected_version
    if type(amount) is not int or abs(amount) > SAFE:
        return {"status": "out_of_range", "retryable": False}
    if key_mode == "generation_bound" and request_generation != meta(db, "generation"):
        return {"status": "reconcile_required", "retryable": False}
    namespace = request_generation if key_mode == "generation_bound" else "stable"
    db.execute("BEGIN IMMEDIATE")
    try:
        row = db.execute("SELECT payload,outcome FROM receipts WHERE namespace=? AND client=? AND operation_id=?",
                         (namespace, client, operation_id)).fetchone()
        if row:
            db.execute("COMMIT")
            if row[0] != canonical(payload):
                return {"status": "operation_payload_mismatch", "retryable": False}
            return {"status": "duplicate", "original_outcome": json.loads(row[1])}
        current_version = meta(db,'generation') + ':' + meta(db,'version')
        if expected_version is not None and expected_version != current_version:
            outcome = {'status':'version_conflict','retryable':False,'current_version':current_version}
            db.execute('INSERT INTO receipts VALUES(?,?,?,?,?)',(namespace,client,operation_id,canonical(payload),canonical(outcome)))
            db.execute('COMMIT')
            return outcome
        old = db.execute("SELECT amount FROM contributors WHERE metric='cards' AND day='2026-10-23' AND contributor='manual'").fetchone()
        new = amount if kind == 'manual_set' else (old[0] if old else 0) + amount
        if abs(new) > SAFE:
            db.execute("ROLLBACK")
            return {"status": "out_of_range", "retryable": False}
        db.execute("INSERT INTO contributors VALUES('cards','2026-10-23','manual',?) ON CONFLICT(metric,day,contributor) DO UPDATE SET amount=excluded.amount", (new,))
        if fail_at == "after_effect":
            raise RuntimeError("injected failure before receipt")
        bump(db)
        outcome = {"status": "applied", "manual_contribution": new}
        db.execute("INSERT INTO receipts VALUES(?,?,?,?,?)", (namespace, client, operation_id, canonical(payload), canonical(outcome)))
        db.execute("COMMIT")
        if fail_at == "after_commit":
            return {"status": "response_lost", "client_outcome": "unknown"}
        return outcome
    except RuntimeError as error:
        db.execute("ROLLBACK")
        return {"status": "rolled_back", "reason": str(error)}

def receipt_scenarios(work):
    rows = []
    for mode in ("stable_client_identity", "generation_bound"):
        for schedule in ("response_lost", "before_receipt_failure", "restore_before_unacknowledged",
                         "restore_after_unacknowledged", "acknowledged_after_backup"):
            name = mode + "-" + schedule
            path, snap = work / (name + ".db"), work / (name + ".backup")
            db = open_server(path)
            client_path = work / (name + '-client.db')
            client = sqlite3.connect(client_path)
            client.execute('CREATE TABLE outbox(operation_id TEXT PRIMARY KEY,amount INTEGER)')
            client.execute("INSERT INTO outbox VALUES('operation-1',5)")
            client.commit()
            trace = []
            if schedule in ("restore_before_unacknowledged", "acknowledged_after_backup"):
                backup(db, snap)
            fail_at = (None if schedule == 'acknowledged_after_backup' else
                       'after_effect' if schedule == 'before_receipt_failure' else 'after_commit')
            trace.append(manual_add(db, mode, "server-a", "phone-1", "operation-1", 5, fail_at))
            if trace[-1]['status'] == 'applied':
                client.execute("DELETE FROM outbox WHERE operation_id='operation-1'")
                client.commit()
            client.close()
            if schedule == "restore_after_unacknowledged":
                backup(db, snap)
            if schedule.startswith("restore_") or schedule == "acknowledged_after_backup":
                db.close()
                db = restored(path, snap)
            else:
                db.close()
                db = open_server(path)
            client = sqlite3.connect(client_path)
            if client.execute('SELECT COUNT(*) FROM outbox').fetchone()[0]:
                trace.append(manual_add(db, mode, "server-a", "phone-1", "operation-1", 5))
                if trace[-1]['status'] in ('applied','duplicate'):
                    client.execute("DELETE FROM outbox WHERE operation_id='operation-1'")
                    client.commit()
            rows.append({"variant": mode, "scenario": schedule, "trace": trace,
                         "total_after": total(db), "receipt_rows": db.execute("SELECT COUNT(*) FROM receipts").fetchone()[0],
                         "persisted_pending_commands": client.execute('SELECT COUNT(*) FROM outbox').fetchone()[0],
                         "observation": ("Acknowledged operation already removed from outbox; neither design can replay it."
                                         if schedule == "acknowledged_after_backup" else
                                         "Effect and receipt reopened or restored together in SQLite.")})
            client.close()
            db.close()
        path = work / (mode + "-payload.db")
        db = open_server(path)
        trace = [manual_add(db, mode, "server-a", "phone-1", "same-id", 5),
                 manual_add(db, mode, "server-a", "phone-1", "same-id", 6)]
        rows.append({"variant": mode, "scenario": "same_id_changed_payload", "trace": trace, "total_after": total(db)})
        db.close()
    path, snap = work / "unsafe-rekey.db", work / "unsafe-rekey.backup"
    db = open_server(path)
    first = manual_add(db, "generation_bound", "server-a", "phone-1", "operation-1", 5, "after_commit")
    backup(db, snap)
    db.close()
    db = restored(path, snap)
    second = manual_add(db, "generation_bound", "server-b", "phone-1", "operation-1", 5)
    rows.append({"variant": "unsafe_client_rekeys_on_restore", "scenario": "preserved_effect_new_namespace",
                 "trace": [first, second], "total_after": total(db),
                 "observation": "Counterexample to blindly rebinding IDs; the strict generation-bound candidate pauses instead."})
    db.close()
    return rows

def edit_scenarios(work):
    rows = []
    db = open_server(work/'conditional-edits.db')
    trace = [manual_add(db,'stable_client_identity','server-a','phone-1','set-1',4,kind='manual_set',expected_version='server-a:0'),
             manual_add(db,'stable_client_identity','server-a','phone-2','set-2',7,kind='manual_set',expected_version='server-a:0'),
             manual_add(db,'stable_client_identity','server-a','phone-1','set-1',4,kind='manual_set',expected_version='server-a:0'),
             manual_add(db,'stable_client_identity','server-a','phone-2','set-2',7,kind='manual_set',expected_version='server-a:1')]
    rows.append({'variant':'stable_identity_opaque_precondition','scenario':'two_devices_conditional_set',
                 'trace':trace,'total_after':total(db),
                 'note':'Conflict outcome binds immutable payload too; resolving it requires a new operation ID.'})
    db.close()
    path,snap=work/'restore-conditional.db',work/'restore-conditional.backup'
    db=open_server(path)
    manual_add(db,'stable_client_identity','server-a','phone-1','seed',5)
    backup(db,snap)
    trace=[manual_add(db,'stable_client_identity','server-a','phone-1','set-lost',7,'after_commit',kind='manual_set',expected_version='server-a:1')]
    db.close()
    db=restored(path,snap)
    trace.append(manual_add(db,'stable_client_identity','server-a','phone-1','set-lost',7,kind='manual_set',expected_version='server-a:1'))
    rows.append({'variant':'stable_identity_opaque_precondition','scenario':'restore_conditional_edit_without_receipt',
                 'trace':trace,'total_after':total(db),
                 'note':'Stable ID does not waive edit preconditions. Generation-bearing version rejects a potentially different restored base.'})
    db.close()
    return rows

def source_update(db, stream, revision, amount, epoch="epoch-a"):
    db.execute("INSERT OR IGNORE INTO authorities VALUES(?,'epoch-a',0)", (stream,))
    authority = db.execute('SELECT epoch,needs_reconciliation FROM authorities WHERE stream=?',(stream,)).fetchone()
    if epoch != authority[0]:
        return {"status": "fenced"}
    if authority[1]:
        return {"status": "needs_reconciliation"}
    payload = canonical({"amount": amount, "cleared": amount == 0})
    row = db.execute("SELECT epoch,revision,payload FROM source_revisions WHERE stream=? AND day='2026-10-23'", (stream,)).fetchone()
    if row and row[0] == epoch:
        if revision < row[1]:
            return {"status": "stale"}
        if revision == row[1]:
            return {"status": "unchanged" if payload == row[2] else "revision_payload_conflict"}
    db.execute("BEGIN IMMEDIATE")
    try:
        db.execute("INSERT INTO source_revisions VALUES(?,'2026-10-23',?,?,?,?) ON CONFLICT(stream,day) DO UPDATE SET epoch=excluded.epoch,revision=excluded.revision,payload=excluded.payload,cleared=excluded.cleared",
                   (stream, epoch, revision, payload, int(amount == 0)))
        db.execute("INSERT INTO contributors VALUES('cards','2026-10-23',?,?) ON CONFLICT(metric,day,contributor) DO UPDATE SET amount=excluded.amount", (stream, amount))
        bump(db)
        db.execute("COMMIT")
    except Exception:
        db.execute("ROLLBACK")
        raise
    return {"status": "applied"}

def handoff(db, stream, epoch, candidate_amount, unchecked=False):
    accepted = db.execute("SELECT payload FROM source_revisions WHERE stream=? AND day='2026-10-23'",(stream,)).fetchone()
    candidate = canonical({'amount':candidate_amount,'cleared':candidate_amount==0})
    mismatch = bool(accepted and candidate != accepted[0])
    pending = mismatch and not unchecked
    db.execute('INSERT INTO authorities VALUES(?,?,?) ON CONFLICT(stream) DO UPDATE SET epoch=excluded.epoch,needs_reconciliation=excluded.needs_reconciliation',
               (stream,epoch,int(pending)))
    return {'status':'needs_reconciliation' if pending else 'authority_granted',
            'candidate_matches_accepted_payload':not mismatch,
            'policy':'unchecked_reset' if unchecked else 'compare_accepted_day_before_enabling'}

def ordering_scenarios(work):
    rows = []
    for variant in ("persisted_counter", "clock_revision", "epoch_with_reconciliation", "epoch_without_reconciliation"):
        db = open_server(work / (variant + ".db"))
        trace = [source_update(db, "yomikata", 1000, 12),
                 source_update(db, "yomikata", 1001, 0)]
        if variant == "persisted_counter":
            trace.append(source_update(db, "yomikata", 11, 10))
        elif variant == "clock_revision":
            trace.append(source_update(db, "yomikata", max(10 + 1, 2000), 10))
        else:
            trace.append(handoff(db,'yomikata','epoch-b',10,unchecked=variant=='epoch_without_reconciliation'))
            trace.append(source_update(db, "yomikata", 1, 10, epoch="epoch-b"))
            trace.append(source_update(db, "yomikata", 1002, 99, epoch="epoch-a"))
            if variant == 'epoch_with_reconciliation':
                trace.append(handoff(db,'yomikata','epoch-b',0))
                trace.append(source_update(db,'yomikata',1,0,epoch='epoch-b'))
        rows.append({"variant": variant, "scenario": "older_local_snapshot_after_clear",
                     "trace": trace, "total_after": total(db),
                     "tombstone": dict(zip(("epoch","revision","payload","cleared"),
                                          db.execute("SELECT epoch,revision,payload,cleared FROM source_revisions").fetchone()))})
        db.close()
    db = open_server(work / "clock-rollback.db")
    trace = [source_update(db, "yomikata", 1001, 0),
             source_update(db, "yomikata", max(10 + 1, 900), 13)]
    rows.append({"variant": "clock_revision", "scenario": "clock_rollback_and_lost_local_counter",
                 "trace": trace, "total_after": total(db)})
    trace = [source_update(db, "yomikata", 1001, 0),
             source_update(db, "yomikata", 1001, 8),
             source_update(db, "yomikata", 1000, 8)]
    rows.append({"variant": "revision_comparison", "scenario": "same_clear_changed_payload_late_update",
                 "trace": trace, "total_after": total(db)})
    db.close()
    return rows

def snapshot(db, scope, today, variant, if_none_match=None):
    rows = [{"metric": metric, "day": day, "contributor": contributor, "amount": amount}
            for metric, day, contributor, amount in db.execute(
                "SELECT metric,day,contributor,amount FROM contributors WHERE day>=? AND day<=? ORDER BY metric,day,contributor", scope)]
    body = {"scope": list(scope), "schema": "daily-v1", "home_timezone": "Europe/Berlin",
            "home_today": today, "generation": meta(db,"generation"), "rows": rows}
    etag = meta(db,"version") if variant == "database_version_only" else digest(body)
    etag = '"' + etag + '"'
    return {"status": 304 if if_none_match == etag else 200, "etag": etag, "body": body}

def open_client(path):
    client = sqlite3.connect(path, isolation_level=None)
    client.executescript("""
        CREATE TABLE scopes(scope TEXT PRIMARY KEY,seq INTEGER,etag TEXT,body TEXT);
        CREATE TABLE outbox(client TEXT,operation_id TEXT,payload TEXT,PRIMARY KEY(client,operation_id));
    """)
    client.execute("INSERT INTO outbox VALUES('phone-1','pending-1',?)", (canonical({"kind":"manual_add","amount":3}),))
    return client

def install(client, requested_scope, seq, response):
    key = canonical(list(requested_scope))
    current = client.execute("SELECT seq FROM scopes WHERE scope=?", (key,)).fetchone()
    if current and seq < current[0]:
        return "ignored_older_response"
    if response["status"] == 304:
        return "kept_cached_scope"
    if response["body"]["scope"] != list(requested_scope):
        return "rejected_scope_mismatch"
    client.execute("BEGIN IMMEDIATE")
    client.execute("INSERT INTO scopes VALUES(?,?,?,?) ON CONFLICT(scope) DO UPDATE SET seq=excluded.seq,etag=excluded.etag,body=excluded.body",
                   (key,seq,response["etag"],canonical(response["body"])))
    client.execute("COMMIT")
    return "replaced_complete_scope"

def cache_scenarios(work):
    rows = []
    scope = ("2026-10-23","2026-10-23")
    for variant in ("database_version_only", "representation_etag"):
        db = open_server(work / (variant+".db"))
        source_update(db,"yomikata",1,10)
        first = snapshot(db,scope,"2026-10-23",variant)
        next_day = snapshot(db,scope,"2026-10-24",variant,first["etag"])
        next_scope = snapshot(db,("2026-10-24","2026-10-24"),"2026-10-24",variant,first["etag"])
        db.execute("UPDATE metadata SET value='server-b' WHERE key='generation'")
        restored_view = snapshot(db,scope,"2026-10-23",variant,first["etag"])
        rows.append({"variant":variant,"scenario":"rollover_scope_generation",
                     "initial":first,"rollover":next_day,"changed_scope":next_scope,"changed_generation":restored_view,
                     "note":"304 bodies shown only for instrumentation. Cross-scope validator reuse is an intentionally incorrect client; URL/scope-specific cache keys also prevent it. Rollover concerns views that display home_today, not every historical series."})
        client = open_client(work / (variant+"-client.db"))
        trace = [install(client,scope,1,first)]
        source_update(db,"yomikata",2,12)
        newer = snapshot(db,scope,"2026-10-23",variant)
        trace.append(install(client,scope,3,newer))
        trace.append(install(client,scope,2,first))
        db.execute("BEGIN IMMEDIATE")
        db.execute("DELETE FROM contributors WHERE contributor='yomikata'")
        bump(db)
        db.execute("COMMIT")
        deleted = snapshot(db,scope,"2026-10-23",variant,newer["etag"])
        trace.append(install(client,scope,4,deleted))
        cached = json.loads(client.execute("SELECT body FROM scopes WHERE scope=?", (canonical(list(scope)),)).fetchone()[0])
        client.close()
        client = sqlite3.connect(work / (variant+"-client.db"))
        rows.append({"variant":variant,"scenario":"deletion_racing_response_pending_queue",
                     "trace":trace,"cached_rows_after_delete":cached["rows"],
                     "persisted_pending_commands":client.execute("SELECT COUNT(*) FROM outbox").fetchone()[0]})
        client.close()
        db.close()
    return rows + http_cache_scenarios(work)

def http_cache_scenarios(work):
    rows = []
    for variant in ('database_version_only','representation_etag'):
        path=work/(variant+'-http.db')
        db=open_server(path)
        source_update(db,'yomikata',1,10)
        today=['2026-10-23']
        class Handler(BaseHTTPRequestHandler):
            def do_GET(self):
                query=parse_qs(urlparse(self.path).query)
                scope=(query['from'][0],query['to'][0])
                reader=sqlite3.connect(path)
                response=snapshot(reader,scope,today[0],variant,self.headers.get('If-None-Match'))
                reader.close()
                data=canonical(response['body']).encode() if response['status']==200 else b''
                self.send_response(response['status'])
                self.send_header('ETag',response['etag'])
                self.send_header('Content-Length',str(len(data)))
                if data:
                    self.send_header('Content-Type','application/json')
                self.end_headers()
                self.wfile.write(data)
            def log_message(self,*args):
                pass
        server=HTTPServer(('127.0.0.1',0),Handler)
        thread=threading.Thread(target=server.serve_forever,kwargs={'poll_interval':0.01},daemon=True)
        thread.start()
        cache={}
        def fetch(day):
            url=f'http://127.0.0.1:{server.server_port}/daily?from={day}&to={day}'
            etag=cache.get(url)
            headers={'If-None-Match':etag} if etag else {}
            request=Request(url,headers=headers)
            try:
                response=urlopen(request,timeout=5)
            except HTTPError as error:
                response=error
            with response:
                data=response.read()
                received_etag=response.headers.get('ETag')
                cache[url]=received_etag
                return {'status':response.status,'sent_validator':bool(etag),
                        'body_present':bool(data),'body':json.loads(data) if data else None}
        try:
            trace=[fetch('2026-10-23'),fetch('2026-10-23')]
            today[0]='2026-10-24'
            trace.append(fetch('2026-10-23'))
            trace.append(fetch('2026-10-24'))
            rows.append({'variant':variant,'scenario':'actual_http_conditional_get_and_url_scoping',
                         'trace':trace,
                         'note':'Real ephemeral localhost HTTP server and urllib client; second request is unchanged, third crosses rollover, fourth has a distinct URL and no borrowed validator.'})
        finally:
            server.shutdown()
            thread.join(timeout=5)
            server.server_close()
            db.close()
    return rows

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--variant", choices=["all","receipts","edits","ordering","cache"], default="all")
    parser.add_argument("--output", type=Path, default=Path(__file__).with_name("results.json"))
    args = parser.parse_args()
    with tempfile.TemporaryDirectory(prefix="lifestats-sync-prototype-") as temp:
        work = Path(temp)
        groups = {"receipts":receipt_scenarios,"edits":edit_scenarios,"ordering":ordering_scenarios,"cache":cache_scenarios}
        outcomes = {key:run(work) for key,run in groups.items() if args.variant in ("all",key)}
    report = {"source_revision":BASELINE,"python_sqlite_version":sqlite3.sqlite_version,
              "scope":"Synthetic throwaway model; SQLite commit/rollback, file reopen and backup API plus cache-only loopback HTTP are real. No actual process kill, concurrent writer race, production adapter, RN runtime or production DB.",
              "data_shape":"Contributor rows plus immutable client operations; source revision tombstones; cached scopes separate from outbox.",
              "outcomes":outcomes}
    args.output.parent.mkdir(parents=True,exist_ok=True)
    args.output.write_text(json.dumps(report,indent=2)+"\n")
    for group, cases in outcomes.items():
        for case in cases:
            summary = {key:case[key] for key in ("variant","scenario","total_after","trace","persisted_pending_commands") if key in case}
            if group == "cache" and case["scenario"] == "rollover_scope_generation":
                summary["http_model_statuses"] = {key:case[key]["status"] for key in ("initial","rollover","changed_scope","changed_generation")}
            print(canonical({"group":group,**summary}))
    print("Observed results written to",args.output)

if __name__ == "__main__":
    main()


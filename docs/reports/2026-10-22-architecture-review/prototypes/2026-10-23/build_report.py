#!/usr/bin/env python3
"""Build a self-contained viewer from recorded prototype observations."""
import json
from pathlib import Path

ROOT=Path(__file__).resolve().parent
def read(name):
    return json.loads((ROOT/name).read_text())

def main():
    storage=read('storage/results.json')
    api=read('api/results.json')
    sync=read('sync/results.json')
    summary=read('summary.json')
    storage_cases=[]
    for variant in storage['variants']:
        for scenario in variant['scenarios']:
            storage_cases.append({
                'variant':variant['variant'],'scenario':scenario['name'],
                'steps':[{'label':step['label'],'commands':step['commands'],
                          'outcomes':step['outcomes'],'commit':step['commit'],
                          'values':step['observation']['views']}
                         for step in scenario['steps']],
                'last_stored_rows':scenario['steps'][-1]['observation']['stored_rows']})
    api_examples={'unknown_top','unknown_value','suppression_with_value','duplicate_value','date_impossible','date_not_padded','safe_float_near_safe','raw_i64_positive_rounded','raw_i64_i64_max','decimal_above_i64','opaque_empty'}
    api_view={key:api[key] for key in ('status','source_revision','versions','verification','summary','additional_authority')}
    api_view['cases']=[case for case in api['cases'] if case['id'] in api_examples]
    data={'summary':summary,'storage_cases':storage_cases,
          'storage_encoding':storage['encoding_boundary'],
          'storage_intervals':storage['interval_policy'],
          'api':api_view,'offline_cases':sum((sync['outcomes'][key] for key in ('receipts','edits','ordering')),[]),
          'cache_cases':sync['outcomes']['cache']}
    encoded=json.dumps(data,ensure_ascii=False).replace('<','\\u003c')
    page=HTML.replace('__DATA__',encoded)
    (ROOT/'report.html').write_text(page)
    print(json.dumps({'output':str(ROOT/'report.html'),'storage_cases':len(storage_cases),
                      'offline_cases':len(data['offline_cases']),'cache_cases':len(data['cache_cases'])}))

HTML=r"""<!doctype html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1"><title>Lifestats prototype decisions</title><style>
:root{font-family:var(--font-sans,system-ui,sans-serif);color:var(--foreground,#20252b);font-size:15px;line-height:1.5}
*{box-sizing:border-box}body{margin:0;background:transparent}h2{font-size:1.65rem;line-height:1.18;letter-spacing:-.025em;margin:.3rem 0 .8rem}h3{font-size:1.15rem;margin:1.1rem 0 .5rem}p{margin:.5rem 0 .9rem}.meta{color:var(--muted-foreground,#66717b);font-size:.82rem}.lead{font-size:1.03rem;max-width:65ch}
nav{display:flex;flex-wrap:wrap;gap:6px;border-bottom:1px solid var(--border,#d8dee5);padding:12px 0;margin:14px 0}
button,select{font:inherit;color:inherit;background:var(--secondary,#edf1f5);border:1px solid var(--border,#d8dee5);border-radius:7px;padding:8px 12px}
button{cursor:pointer}button[aria-selected=true]{color:var(--primary-foreground,#fff);background:var(--primary,#204f72);border-color:transparent}button:focus-visible,select:focus-visible{outline:2px solid var(--ring,#5286b4);outline-offset:2px}
.panel[hidden]{display:none}.cards{display:grid;grid-template-columns:1fr 1fr;gap:10px}.card{padding:16px;background:var(--card,#f3f5f7);border:1px solid var(--border,#d8dee5);border-radius:var(--radius,10px)}.card h3{margin:0 0 6px}.card p{margin:0}
.tag{display:inline-block;margin:0 0 8px;border-radius:20px;padding:3px 8px;font-size:.74rem;background:var(--accent-surface,#e2ecf3);color:var(--accent-surface-foreground,#234c68)}
.note{padding:16px;border-left:3px solid var(--warning,#b78123);background:var(--warning-surface,#fbf3e3);border-radius:var(--radius,8px);margin:14px 0}.note p:last-child{margin-bottom:0}
ul{padding-left:20px}li{margin:6px 0}.controls{display:flex;align-items:center;gap:10px;flex-wrap:wrap;margin:14px 0}select{max-width:100%;min-width:0;flex:1}
pre{padding:16px;background:var(--code-background,#f0f3f6);color:var(--code-foreground,inherit);border-radius:var(--radius,8px);font-family:var(--font-mono,monospace);font-size:.79rem;white-space:pre-wrap;overflow-wrap:anywhere;max-height:510px;overflow:auto;margin:10px 0}
details{border-top:1px solid var(--border,#d8dee5);padding:12px 0}summary{cursor:pointer;font-weight:600}.footer{border-top:1px solid var(--border,#d8dee5);margin-top:18px;padding:14px 0}.state{font-size:.8rem;font-weight:600;color:var(--muted-foreground,#66717b)}a{color:var(--accent,#245c80)}
@media(max-width:480px){.cards{grid-template-columns:1fr}h2{font-size:1.4rem}button{font-size:.86rem;padding:8px 10px}.controls{display:block}select{width:100%;margin-top:6px}}
</style></head><body>
<p class="meta">23 October 2026 · synthetic architecture experiments · source 6079789</p>
<h2>Keep the boundaries. Simplify the protocol.</h2>
<p class="lead" id="lead"></p>
<nav role="tablist" aria-label="Prototype evidence">
<button role="tab" aria-controls="decisions" aria-selected="true" id="tab-decisions">Decisions</button>
<button role="tab" aria-controls="storage" aria-selected="false" id="tab-storage">Storage</button>
<button role="tab" aria-controls="api" aria-selected="false" id="tab-api">API</button>
<button role="tab" aria-controls="offline" aria-selected="false" id="tab-offline">Offline</button>
<button role="tab" aria-controls="cache" aria-selected="false" id="tab-cache">Cache</button>
</nav>
<section class="panel" role="tabpanel" id="decisions" aria-labelledby="tab-decisions">
<div class="cards" id="choices"></div>
<div class="note"><strong>Product choices remain explicit.</strong><p id="product"></p></div>
<h3>Next implementation proof</h3><p id="next"></p>
<h3>What these runs establish</h3><p id="limits"></p>
</section>
<section class="panel" role="tabpanel" id="storage" aria-labelledby="tab-storage" hidden>
<h3>Rows for contributions, without invented provenance</h3>
<p>Three SQLite representations use the same semantic reducer. This compares storage capabilities; it is not three independent checks of the arithmetic.</p>
<p>Setting a manual contribution to 4 gives 16 after a source changes from 10 to 12. Explicitly setting the aggregate to 4 gives 6 after that correction. The commands have different meanings.</p>
<div class="controls"><label for="storage-case">Observed scenario</label><select id="storage-case"></select></div><pre id="storage-output"></pre>
<details><summary>Exact legacy encoding boundary</summary><pre id="storage-encoding"></pre></details>
<details><summary>Metric policy: sum versus interval union</summary><pre id="storage-intervals"></pre></details>
</section>
<section class="panel" role="tabpanel" id="api" aria-labelledby="tab-api" hidden>
<h3>Rust schemas and generated TypeScript</h3><p id="api-summary"></p>
<div id="api-lessons"></div>
<details open><summary>Recorded toolchain observations</summary><pre id="api-output"></pre></details>
</section>
<section class="panel" role="tabpanel" id="offline" aria-labelledby="tab-offline" hidden>
<h3>Operation identity, ordering and restore</h3>
<p>SQLite effect/receipt transactions, persisted queues, file reopen and backup restoration were executed. Schedules are deterministic, with no concurrent writers or process-kill proof.</p>
<p>Stable operation IDs replay additive writes without rebinding them to a new server generation. Conditional edits still check a generation-bearing version. Clock revisions and unchecked authority changes can both revive stale content.</p>
<div class="controls"><label for="offline-case">Observed scenario</label><select id="offline-case"></select></div><pre id="offline-output"></pre>
</section>
<section class="panel" role="tabpanel" id="cache" aria-labelledby="tab-cache" hidden>
<h3>Complete scope refreshes with correct validators</h3>
<p>Complete replacement removed deleted cached rows and preserved the pending queue. Actual loopback HTTP returned a body-free 304 for unchanged views. A representation-aware validator refreshed a today-sensitive view at day rollover.</p>
<p>URL-specific cache keys prevent borrowing another scope's validator. A database version alone misses time-dependent output and restore generations. No bandwidth, battery or latency benefit was measured.</p>
<div class="controls"><label for="cache-case">Observed scenario</label><select id="cache-case"></select></div><pre id="cache-output"></pre>
</section>
<div class="footer"><p class="meta" id="review"></p><p class="meta">Throwaway prototypes. Production code, live databases and services were unchanged. Source runners, lockfiles and observations accompany this report.</p></div>
<script id="observations" type="application/json">__DATA__</script>
<script>
const data=JSON.parse(document.getElementById('observations').textContent);
const text=(id,value)=>document.getElementById(id).textContent=value;
text('lead',data.summary.lead);
text('product',data.summary.product_choices);
text('next',data.summary.next);
text('limits',data.summary.limits);
text('review',data.summary.review);
text('api-summary',data.summary.api_summary);
for(const choice of data.summary.choices){
 const node=document.createElement('article');node.className='card';
 const tag=document.createElement('span');tag.className='tag';tag.textContent=choice.status;
 const title=document.createElement('h3');title.textContent=choice.title;
 const copy=document.createElement('p');copy.textContent=choice.text;
 node.append(tag,title,copy);document.getElementById('choices').append(node);
}
for(const lesson of data.summary.api_lessons){
 const p=document.createElement('p');p.textContent=lesson;document.getElementById('api-lessons').append(p);
}
function bindCases(prefix,cases){
 const select=document.getElementById(prefix+'-case');
 cases.forEach((entry,index)=>{const opt=document.createElement('option');opt.value=index;opt.textContent=entry.variant+' / '+entry.scenario;select.append(opt);});
 const update=()=>text(prefix+'-output',JSON.stringify(cases[Number(select.value)],null,2));
 select.addEventListener('change',update);update();
}
bindCases('storage',data.storage_cases);bindCases('offline',data.offline_cases);bindCases('cache',data.cache_cases);
text('storage-encoding',JSON.stringify(data.storage_encoding,null,2));
text('storage-intervals',JSON.stringify(data.storage_intervals,null,2));
text('api-output',JSON.stringify(data.api,null,2));
const tabs=[...document.querySelectorAll('[role=tab]')];
function selectTab(tab){
 for(const other of tabs){const active=other===tab;other.setAttribute('aria-selected',String(active));other.tabIndex=active?0:-1;document.getElementById(other.getAttribute('aria-controls')).hidden=!active;}
}
tabs.forEach((tab,index)=>{tab.addEventListener('click',()=>selectTab(tab));tab.addEventListener('keydown',event=>{if(!['ArrowLeft','ArrowRight'].includes(event.key))return;event.preventDefault();const next=tabs[(index+(event.key==='ArrowRight'?1:-1)+tabs.length)%tabs.length];selectTab(next);next.focus();});});
selectTab(tabs[0]);
console.log('Prototype viewer ready', {tabs:tabs.length,storageCases:data.storage_cases.length,offlineCases:data.offline_cases.length,cacheCases:data.cache_cases.length});
</script></body></html>"""

if __name__=='__main__':
    main()


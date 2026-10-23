import json,sys
r=json.load(open(sys.argv[1]))
pat=sys.argv[2:]
def g(d,k):
    return None if d is None else d.get(k)
for c in r['cases']:
    if not any(p in c['id'] for p in pat): continue
    print('==',c['id'],c['root'])
    print('  raw     ',c['raw'][:220])
    print('  rust    ',g(c['rust'],'accepted'), (g(c['rust'],'round_trip_raw') or json.dumps(g(c['rust'],'error')))[:220])
    print('  js_rt   ',(g(c['js_parse'],'round_trip_raw') or '')[:220])
    for p in ('ajv','zod'):
        x=c.get(p)
        if isinstance(x,dict) and 'root' in json.dumps(list(x.keys())): pass
        print('  %-8s'%p, json.dumps(x)[:300])
    extra={k:v for k,v in c.items() if k not in ('id','root','raw','rust','js_parse','ajv','zod')}
    if extra: print('  extra   ',json.dumps(extra)[:500])

import { useEffect, useMemo, useState } from 'react';
import { Button } from './ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from './ui/card';
import { Input } from './ui/input';
import { Label } from './ui/label';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from './ui/select';
import { Combobox } from './ui/combobox';
import { api } from '../lib/api';
import { useFunctions } from '../hooks/useFunctions';

type Route = { route_id: string; path: string; method?: string; function_name: string; created_at: string };

export function ApiGateway() {
  const [routes, setRoutes] = useState<Route[]>([]);
  const [loading, setLoading] = useState(true);
  const [form, setForm] = useState<{ path: string; method: string; function_name: string }>({ path: '/example', method: 'ANY', function_name: '' });
  const { data: functionsData, isLoading: fnsLoading } = useFunctions();
  const [fnFilter, setFnFilter] = useState('');
  const fnNames = (functionsData?.functions ?? []).map(f=>f.function_name).sort();
  const filteredFnNames = useMemo(() => fnNames.filter(n => n.toLowerCase().includes(fnFilter.toLowerCase())), [fnNames, fnFilter]);
  const [err, setErr] = useState<string>('');

  const fetchRoutes = async ()=>{
    setLoading(true);
    try { const data = await api.listApiRoutes(); setRoutes(data.routes); } finally { setLoading(false); }
  };
  useEffect(()=>{ fetchRoutes(); },[]);

  const addRoute = async ()=>{
    setErr('');
    const path = form.path.trim();
    if (!path.startsWith('/')) { setErr('Path must start with /'); return; }
    if (!form.function_name) { setErr('Function name required'); return; }
    const method = form.method === 'ANY' ? undefined : form.method;
    await api.createApiRoute({ path, function_name: form.function_name, method });
    setForm({ path: '/example', method: 'ANY', function_name: '' });
    fetchRoutes();
  };
  const delRoute = async (id: string)=>{ await api.deleteApiRoute(id); fetchRoutes(); };

  // Simple tester
  const [tMethod, setTMethod] = useState('GET');
  const [tPath, setTPath] = useState('/');
  const [headers, setHeaders] = useState<{key:string;value:string}[]>([{key:'Content-Type', value:'application/json'}]);
  const [query, setQuery] = useState<{key:string;value:string}[]>([{key:'', value:''}]);
  const [body, setBody] = useState('');
  const [resp, setResp] = useState<{status:number; headers:Record<string,string>; bodyText:string} | null>(null);
  const test = async ()=>{
    const hdrs: Record<string,string> = {}; headers.filter(h=>h.key).forEach(h=>hdrs[h.key]=h.value);
    const qs: Record<string,string> = {}; query.filter(q=>q.key).forEach(q=>qs[q.key]=q.value);
    const r = await api.requestPath(tPath || '/', { method: tMethod, headers: hdrs, query: qs, body });
    setResp(r);
  };

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-3xl font-bold tracking-tight">API Gateway</h1>
          <p className="text-muted-foreground">Define path → function mappings and test them</p>
        </div>
        <Button onClick={fetchRoutes} variant="outline">Refresh</Button>
      </div>

      <Card>
        <CardHeader>
          <CardTitle>Create Mapping</CardTitle>
          <CardDescription>Map a path prefix to a function. Longest-prefix wins. Method optional.</CardDescription>
        </CardHeader>
        <CardContent className="space-y-3">
          {err && <div className="text-sm text-red-600">{err}</div>}
          <div className="grid grid-cols-3 gap-2">
            <div>
              <Label>Path Prefix</Label>
              <Input value={form.path} onChange={e=>setForm({...form, path: e.target.value})} placeholder="/api" />
            </div>
            <div>
              <Label>Method</Label>
              <Select value={form.method} onValueChange={(v)=>setForm({...form, method: v})}>
                <SelectTrigger className="w-full"><SelectValue /></SelectTrigger>
                <SelectContent>
                  {['ANY','GET','POST','PUT','DELETE','PATCH'].map(m=> <SelectItem key={m} value={m}>{m}</SelectItem>)}
                </SelectContent>
              </Select>
            </div>
            <div>
              <Label>Function</Label>
              <Combobox
                value={form.function_name}
                onChange={(v)=>setForm({...form, function_name: v})}
                options={fnNames}
                placeholder={fnsLoading ? 'Loading…' : 'Select function'}
                searchPlaceholder="Search functions"
              />
            </div>
          </div>
          <div className="flex justify-end">
            <Button onClick={addRoute}>Add Mapping</Button>
          </div>
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>Mappings</CardTitle>
          <CardDescription>Existing path mappings</CardDescription>
        </CardHeader>
        <CardContent>
          {loading ? (
            <div className="text-sm text-muted-foreground">Loading…</div>
          ) : routes.length === 0 ? (
            <div className="text-sm text-muted-foreground">No routes</div>
          ) : (
            <div className="border rounded overflow-auto">
              <table className="w-full text-sm">
                <thead>
                  <tr className="bg-gray-50 text-left">
                    <th className="px-2 py-1">Path</th>
                    <th className="px-2 py-1">Method</th>
                    <th className="px-2 py-1">Function</th>
                    <th className="px-2 py-1">Created</th>
                    <th className="px-2 py-1 text-right">Actions</th>
                  </tr>
                </thead>
                <tbody>
                  {routes.map(r=> (
                    <tr key={r.route_id} className="border-t">
                      <td className="px-2 py-1 font-mono">{r.path}</td>
                      <td className="px-2 py-1">{r.method || 'ANY'}</td>
                      <td className="px-2 py-1">{r.function_name}</td>
                      <td className="px-2 py-1 text-xs">{new Date(r.created_at).toLocaleString()}</td>
                      <td className="px-2 py-1 text-right">
                        <Button size="sm" variant="outline" onClick={()=>delRoute(r.route_id)}>Delete</Button>
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          )}
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>Route Tester</CardTitle>
          <CardDescription>Send requests to mapped paths</CardDescription>
        </CardHeader>
        <CardContent className="space-y-3">
          <div className="grid grid-cols-3 gap-2">
            <div>
              <Label>Method</Label>
              <Select value={tMethod} onValueChange={setTMethod}>
                <SelectTrigger className="w-full"><SelectValue /></SelectTrigger>
                <SelectContent>
                  {['GET','POST','PUT','DELETE','PATCH'].map(m=> <SelectItem key={m} value={m}>{m}</SelectItem>)}
                </SelectContent>
              </Select>
            </div>
            <div className="col-span-2">
              <Label>Path</Label>
              <Input value={tPath} onChange={e=>setTPath(e.target.value)} placeholder="/api" />
            </div>
          </div>

          <div>
            <Label>Headers</Label>
            {headers.map((h,idx)=> (
              <div key={idx} className="grid grid-cols-2 gap-2 mt-1">
                <Input placeholder="Header Name" value={h.key} onChange={e=>{ const arr=[...headers]; arr[idx]={...h, key:e.target.value}; setHeaders(arr); }} />
                <Input placeholder="Header Value" value={h.value} onChange={e=>{ const arr=[...headers]; arr[idx]={...h, value:e.target.value}; setHeaders(arr); }} />
              </div>
            ))}
            <div className="mt-2"><Button variant="outline" size="sm" type="button" onClick={()=>setHeaders([...headers,{key:'',value:''}])}>Add Header</Button></div>
          </div>

          <div>
            <Label>Query Params</Label>
            {query.map((q,idx)=> (
              <div key={idx} className="grid grid-cols-2 gap-2 mt-1">
                <Input placeholder="Key" value={q.key} onChange={e=>{ const arr=[...query]; arr[idx]={...q, key:e.target.value}; setQuery(arr); }} />
                <Input placeholder="Value" value={q.value} onChange={e=>{ const arr=[...query]; arr[idx]={...q, value:e.target.value}; setQuery(arr); }} />
              </div>
            ))}
            <div className="mt-2"><Button variant="outline" size="sm" type="button" onClick={()=>setQuery([...query,{key:'',value:''}])}>Add Param</Button></div>
          </div>

          {tMethod !== 'GET' && (
            <div>
              <Label>Body</Label>
              <textarea className="w-full h-32 border rounded p-2 text-sm font-mono" value={body} onChange={e=>setBody(e.target.value)} />
            </div>
          )}

          <div className="flex justify-end">
            <Button onClick={test}>Send</Button>
          </div>

          {resp && (
            <div className="mt-3 border rounded p-2 text-sm">
              <div className="mb-1">Status: <span className="font-medium">{resp.status}</span></div>
              <div className="mb-1">Headers:</div>
              <pre className="bg-gray-50 p-2 rounded max-h-40 overflow-auto">{JSON.stringify(resp.headers, null, 2)}</pre>
              <div className="mt-1">Body:</div>
              <pre className="bg-gray-50 p-2 rounded max-h-64 overflow-auto whitespace-pre-wrap">{resp.bodyText}</pre>
            </div>
          )}
        </CardContent>
      </Card>
    </div>
  );
}

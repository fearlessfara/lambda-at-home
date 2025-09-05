import { useEffect, useState } from 'react';
import { Button } from './ui/button';
import { Input } from './ui/input';
import { Label } from './ui/label';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from './ui/select';
import { api } from '../lib/api';

export function ProxyTester({ functionName }: { functionName: string }) {
  const [method, setMethod] = useState('GET');
  const [pathSuffix, setPathSuffix] = useState('');
  const [headers, setHeaders] = useState<{key:string;value:string}[]>([{key:'Content-Type', value:'application/json'}]);
  const [query, setQuery] = useState<{key:string;value:string}[]>([{key:'', value:''}]);
  const [body, setBody] = useState('');
  const [resp, setResp] = useState<{status:number; headers:Record<string,string>; bodyText:string} | null>(null);
  const [presetName, setPresetName] = useState('');
  const [presets, setPresets] = useState<any[]>([]);
  const [loading, setLoading] = useState(false);

  const onAddHeader = () => setHeaders([...headers, {key:'', value:''}]);
  const onAddQuery = () => setQuery([...query, {key:'', value:''}]);

  const invoke = async () => {
    setLoading(true);
    setResp(null);
    const hdrs: Record<string,string> = {};
    headers.filter(h=>h.key).forEach(h=>{hdrs[h.key]=h.value});
    const qs: Record<string,string> = {};
    query.filter(q=>q.key).forEach(q=>{qs[q.key]=q.value});
    try {
      const r = await api.invokeViaProxy(functionName, { method, pathSuffix, headers: hdrs, query: qs, body });
      setResp(r);
    } finally {
      setLoading(false);
    }
  };


  // Presets
  const lsKey = `proxy-presets:${functionName}`;
  useEffect(()=>{ try { const raw=localStorage.getItem(lsKey); if (raw) setPresets(JSON.parse(raw)); } catch{} },[functionName]);
  const save = (arr:any)=> localStorage.setItem(lsKey, JSON.stringify(arr));
  const onSavePreset = () => {
    if (!presetName) return;
    const data = { method, pathSuffix, headers, query, body };
    const next = [...presets.filter((p:any)=>p.name!==presetName), {name: presetName, data}];
    setPresets(next); save(next);
  };
  const onLoadPreset = (name: string) => {
    const p = presets.find((p:any)=>p.name===name);
    if (p) { setMethod(p.data.method); setPathSuffix(p.data.pathSuffix); setHeaders(p.data.headers); setQuery(p.data.query); setBody(p.data.body); }
  };
  return (
    <div className="space-y-3">
      <div className="grid grid-cols-3 gap-2">
        <div>
          <Label>Method</Label>
          <Select defaultValue={method} onValueChange={setMethod}>
            <SelectTrigger className="w-full"><SelectValue /></SelectTrigger>
            <SelectContent>
              {['GET','POST','PUT','DELETE','PATCH'].map(m=> <SelectItem key={m} value={m}>{m}</SelectItem>)}
            </SelectContent>
          </Select>
        </div>
        <div className="col-span-2">
          <Label>Path Suffix</Label>
          <Input value={pathSuffix} onChange={e=>setPathSuffix(e.target.value)} placeholder="/optional/path" />
        </div>
      </div>

      <div>
        <Label>Headers</Label>
        {headers.map((h,idx)=> (
          <div key={idx} className="grid grid-cols-2 gap-2 mt-1">
            <Input placeholder="Header Name" value={h.key} onChange={e=>{
              const arr=[...headers]; arr[idx]={...h, key:e.target.value}; setHeaders(arr);
            }} />
            <Input placeholder="Header Value" value={h.value} onChange={e=>{
              const arr=[...headers]; arr[idx]={...h, value:e.target.value}; setHeaders(arr);
            }} />
          </div>
        ))}
        <div className="mt-2"><Button variant="outline" size="sm" type="button" onClick={onAddHeader}>Add Header</Button></div>
      </div>
      <div className="grid grid-cols-3 gap-2 items-end">
        <div className="col-span-2">
          <Label>Preset Name</Label>
          <Input value={presetName} onChange={e=>setPresetName(e.target.value)} placeholder="e.g., GET root" />
        </div>
        <div className="flex space-x-2">
          <Button type="button" variant="outline" onClick={onSavePreset}>Save Preset</Button>
          {presets.length>0 && (
            <select className="border rounded px-2 py-1 text-sm" onChange={e=>onLoadPreset(e.target.value)}>
              <option value="">Load preset…</option>
              {presets.map((p:any)=> <option key={p.name} value={p.name}>{p.name}</option>)}
            </select>
          )}
        </div>
      </div>

      <div>
        <Label>Query Params</Label>
        {query.map((q,idx)=> (
          <div key={idx} className="grid grid-cols-2 gap-2 mt-1">
            <Input placeholder="Key" value={q.key} onChange={e=>{
              const arr=[...query]; arr[idx]={...q, key:e.target.value}; setQuery(arr);
            }} />
            <Input placeholder="Value" value={q.value} onChange={e=>{
              const arr=[...query]; arr[idx]={...q, value:e.target.value}; setQuery(arr);
            }} />
          </div>
        ))}
        <div className="mt-2"><Button variant="outline" size="sm" type="button" onClick={onAddQuery}>Add Param</Button></div>
      </div>

      {method !== 'GET' && (
        <div>
          <Label>Body</Label>
          <textarea className="w-full h-32 border rounded p-2 text-sm font-mono" value={body} onChange={e=>setBody(e.target.value)} />
        </div>
      )}

      <div className="flex justify-end">
        <Button onClick={invoke} disabled={loading}>{loading? 'Invoking...' : 'Invoke via Path Proxy'}</Button>
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
    </div>
  );
}

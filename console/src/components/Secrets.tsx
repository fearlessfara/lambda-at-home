import { useEffect, useState } from 'react';
import { Button } from './ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from './ui/card';
import { Input } from './ui/input';
import { Label } from './ui/label';
import { api } from '../lib/api';

export function Secrets() {
  const [list, setList] = useState<{ name: string; created_at: string }[]>([]);
  const [name, setName] = useState('');
  const [value, setValue] = useState('');
  const [loading, setLoading] = useState(false);

  const load = async ()=>{
    setLoading(true);
    try{ const res = await api.listSecrets(); setList(res.secrets); } finally { setLoading(false); }
  };
  useEffect(()=>{ load(); },[]);

  const create = async ()=>{
    if (!name || !value) return;
    await api.createSecret({ name, value });
    setName(''); setValue('');
    load();
  };
  const del = async (n: string)=>{ await api.deleteSecret(n); load(); };

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-3xl font-bold tracking-tight">Secrets</h1>
          <p className="text-muted-foreground">Store and reference secrets in function environment</p>
        </div>
        <Button onClick={load} variant="outline">Refresh</Button>
      </div>

      <Card className="max-w-2xl">
        <CardHeader>
          <CardTitle>Create Secret</CardTitle>
          <CardDescription>Name cannot be retrieved once deleted</CardDescription>
        </CardHeader>
        <CardContent className="space-y-3">
          <div className="grid grid-cols-2 gap-2">
            <div>
              <Label>Name</Label>
              <Input value={name} onChange={e=>setName(e.target.value)} placeholder="DB_PASSWORD" />
            </div>
            <div>
              <Label>Value</Label>
              <Input value={value} onChange={e=>setValue(e.target.value)} placeholder="enter secret value" />
            </div>
          </div>
          <div className="flex justify-end"><Button onClick={create}>Save</Button></div>
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>Secrets</CardTitle>
          <CardDescription>Values are masked and never returned via API</CardDescription>
        </CardHeader>
        <CardContent>
          {loading ? (
            <div className="text-sm text-muted-foreground">Loading…</div>
          ) : list.length === 0 ? (
            <div className="text-sm text-muted-foreground">No secrets</div>
          ) : (
            <div className="border rounded overflow-auto max-w-2xl">
              <table className="w-full text-sm">
                <thead>
                  <tr className="bg-gray-50 text-left">
                    <th className="px-2 py-1">Name</th>
                    <th className="px-2 py-1">Created</th>
                    <th className="px-2 py-1 text-right">Actions</th>
                  </tr>
                </thead>
                <tbody>
                  {list.map(s=> (
                    <tr key={s.name} className="border-t">
                      <td className="px-2 py-1 font-mono">{s.name}</td>
                      <td className="px-2 py-1 text-xs">{new Date(s.created_at).toLocaleString()}</td>
                      <td className="px-2 py-1 text-right"><Button size="sm" variant="outline" onClick={()=>del(s.name)}>Delete</Button></td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          )}
        </CardContent>
      </Card>
    </div>
  );
}

import { useEffect, useState } from 'react';
import { Button } from './ui/button';
import { Input } from './ui/input';
import { Label } from './ui/label';
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from './ui/card';
import { useUpdateFunctionConfiguration } from '../hooks/useFunctions';

export function UpdateFunctionModal({
  open,
  onClose,
  name,
  initial: { handler, timeout, memory_size, environment, description },
}: {
  open: boolean;
  onClose: () => void;
  name: string;
  initial: { handler: string; timeout: number; memory_size: number; environment: Record<string,string>; description?: string };
}) {
  const [local, setLocal] = useState({ handler, timeout, memory_size, description: description || '' });
  const [envRows, setEnvRows] = useState<{key:string;value:string}[]>([]);
  const updateFn = useUpdateFunctionConfiguration();

  useEffect(() => {
    setLocal({ handler, timeout, memory_size, description: description || '' });
    setEnvRows(Object.entries(environment || {}).map(([key,value]) => ({key, value})));
  }, [open, handler, timeout, memory_size, environment, description]);

  const addEnv = () => setEnvRows([...envRows, {key:'', value:''}]);
  const submit = async () => {
    const env: Record<string,string> = {};
    envRows.filter(r=>r.key).forEach(r=> env[r.key]=r.value);
    await updateFn.mutateAsync({ name, data: {
      handler: local.handler,
      timeout: local.timeout,
      memory_size: local.memory_size,
      environment: env,
      description: local.description,
    }});
    onClose();
  };

  if (!open) return null;
  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/40">
      <Card className="w-full max-w-2xl">
        <CardHeader>
          <CardTitle>Update Function Configuration</CardTitle>
          <CardDescription>Handler, memory, timeout, and environment variables</CardDescription>
        </CardHeader>
        <CardContent>
          <div className="grid grid-cols-2 gap-4">
            <div>
              <Label>Handler</Label>
              <Input value={local.handler} onChange={e=>setLocal({...local, handler: e.target.value})} />
            </div>
            <div>
              <Label>Memory (MB)</Label>
              <Input type="number" min={128} max={10240} value={local.memory_size} onChange={e=>setLocal({...local, memory_size: parseInt(e.target.value)||128})} />
            </div>
            <div>
              <Label>Timeout (s)</Label>
              <Input type="number" min={1} max={900} value={local.timeout} onChange={e=>setLocal({...local, timeout: parseInt(e.target.value)||1})} />
            </div>
            <div className="col-span-2">
              <Label>Description</Label>
              <Input value={local.description} onChange={e=>setLocal({...local, description: e.target.value})} />
            </div>
          </div>

          <div className="mt-4">
            <Label>Environment Variables</Label>
            {envRows.map((r,idx)=> (
              <div key={idx} className="grid grid-cols-2 gap-2 mt-1">
                <Input placeholder="KEY" value={r.key} onChange={e=>{ const arr=[...envRows]; arr[idx]={...r, key:e.target.value}; setEnvRows(arr); }} />
                <Input placeholder="value" value={r.value} onChange={e=>{ const arr=[...envRows]; arr[idx]={...r, value:e.target.value}; setEnvRows(arr); }} />
              </div>
            ))}
            <div className="mt-2"><Button variant="outline" size="sm" type="button" onClick={addEnv}>Add variable</Button></div>
          </div>

          <div className="mt-6 flex justify-end space-x-2">
            <Button variant="outline" type="button" onClick={onClose}>Cancel</Button>
            <Button type="button" onClick={submit} disabled={updateFn.isPending}>{updateFn.isPending ? 'Saving...' : 'Save'}</Button>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}


import React, { useEffect, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { Upload, ArrowLeft } from 'lucide-react';
import { Button } from './ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from './ui/card';
import { Input } from './ui/input';
import { Label } from './ui/label';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from './ui/select';
import { useCreateFunction } from '../hooks/useFunctions';
import { useToast } from './ui/use-toast';
import { AVAILABLE_RUNTIMES } from '../types/api';
import { api } from '../lib/api';

export function CreateFunction() {
  const navigate = useNavigate();
  const createFunction = useCreateFunction();
  const { toast } = useToast();
  
  const [formData, setFormData] = useState({
    functionName: '',
    runtime: '',
    handler: '',
    description: '',
    timeout: 3,
    memorySize: 512,
  });
  const [envRows, setEnvRows] = useState<{ key: string; value: string; isSecret?: boolean; secretName?: string }[]>([]);
  const [secrets, setSecrets] = useState<string[]>([]);
  useEffect(()=>{ (async()=>{ try{ const res = await api.listSecrets(); setSecrets(res.secrets.map(s=>s.name)); }catch{} })(); },[]);
  const [zipFile, setZipFile] = useState<File | null>(null);
  const [isUploading, setIsUploading] = useState(false);

  const handleInputChange = (field: string, value: string | number) => {
    setFormData(prev => ({ ...prev, [field]: value }));
  };

  const handleFileChange = (event: React.ChangeEvent<HTMLInputElement>) => {
    const file = event.target.files?.[0];
    if (file) {
      setZipFile(file);
    }
  };

  const convertFileToBase64 = (file: File): Promise<string> => {
    return new Promise((resolve, reject) => {
      const reader = new FileReader();
      reader.readAsDataURL(file);
      reader.onload = () => {
        const result = reader.result as string;
        // Remove the data URL prefix (data:application/zip;base64,)
        const base64 = result.split(',')[1];
        resolve(base64);
      };
      reader.onerror = error => reject(error);
    });
  };

  const handleSubmit = async (event: React.FormEvent) => {
    event.preventDefault();
    
    if (!zipFile) {
      toast({
        title: "Error",
        description: "Please select a ZIP file to upload.",
        variant: "destructive",
      });
      return;
    }

    if (!formData.functionName || !formData.runtime || !formData.handler) {
      toast({
        title: "Error",
        description: "Please fill in all required fields.",
        variant: "destructive",
      });
      return;
    }

    setIsUploading(true);

    try {
      const base64Zip = await convertFileToBase64(zipFile);
      
      const env: Record<string, string> = {};
      envRows.filter(r => r.key).forEach(r => {
        if (r.isSecret && r.secretName) env[r.key] = `SECRET_REF:${r.secretName}`; else env[r.key] = r.value;
      });
      
      const requestData = {
        function_name: formData.functionName,
        runtime: formData.runtime,
        handler: formData.handler,
        code: {
          zip_file: base64Zip,
        },
        description: formData.description || undefined,
        timeout: formData.timeout,
        memory_size: formData.memorySize,
        environment: Object.keys(env).length ? env : undefined,
        publish: true,
      };

      await createFunction.mutateAsync(requestData);
      
      toast({
        title: "Function created",
        description: `Function "${formData.functionName}" has been created successfully.`,
      });
      
      navigate('/functions');
    } catch (error) {
      toast({
        title: "Error",
        description: `Failed to create function: ${error instanceof Error ? error.message : 'Unknown error'}`,
        variant: "destructive",
      });
    } finally {
      setIsUploading(false);
    }
  };

  const getDefaultHandler = (runtime: string) => {
    switch (runtime) {
      case 'nodejs18.x':
        return 'index.handler';
      case 'nodejs22.x':
        return 'index.handler';
      case 'python3.11':
        return 'lambda_function.lambda_handler';
      case 'rust':
        return 'main';
      default:
        return '';
    }
  };

  return (
    <div className="space-y-6">
      <div className="flex items-center space-x-4">
        <Button variant="outline" size="sm" onClick={() => navigate('/functions')}>
          <ArrowLeft className="mr-2 h-4 w-4" />
          Back to Functions
        </Button>
        <div>
          <h1 className="text-3xl font-bold tracking-tight">Create Function</h1>
          <p className="text-muted-foreground">
            Create a new Lambda@Home function
          </p>
        </div>
      </div>

      <Card className="max-w-2xl">
        <CardHeader>
          <CardTitle>Function Configuration</CardTitle>
          <CardDescription>
            Configure your function settings and upload your code
          </CardDescription>
        </CardHeader>
        <CardContent>
          <form onSubmit={handleSubmit} className="space-y-6">
            <div className="grid grid-cols-2 gap-4">
              <div className="space-y-2">
                <Label htmlFor="functionName">Function Name *</Label>
                <Input
                  id="functionName"
                  value={formData.functionName}
                  onChange={(e) => handleInputChange('functionName', e.target.value)}
                  placeholder="my-function"
                  required
                />
              </div>
              <div className="space-y-2">
                <Label htmlFor="runtime">Runtime *</Label>
                <Select
                  value={formData.runtime}
                  onValueChange={(value) => {
                    handleInputChange('runtime', value);
                    handleInputChange('handler', getDefaultHandler(value));
                  }}
                >
                  <SelectTrigger>
                    <SelectValue placeholder="Select runtime" />
                  </SelectTrigger>
                  <SelectContent>
                    {AVAILABLE_RUNTIMES.map((runtime) => (
                      <SelectItem key={runtime} value={runtime}>
                        {runtime}
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
              </div>
            </div>

            <div className="space-y-2">
              <Label htmlFor="handler">Handler *</Label>
              <Input
                id="handler"
                value={formData.handler}
                onChange={(e) => handleInputChange('handler', e.target.value)}
                placeholder="index.handler"
                required
              />
              <p className="text-sm text-muted-foreground">
                The function entry point (e.g., index.handler for Node.js, lambda_function.lambda_handler for Python)
              </p>
            </div>

            <div className="space-y-2">
              <Label htmlFor="description">Description</Label>
              <Input
                id="description"
                value={formData.description}
                onChange={(e) => handleInputChange('description', e.target.value)}
                placeholder="Optional description"
              />
            </div>

            <div className="grid grid-cols-2 gap-4">
              <div className="space-y-2">
                <Label htmlFor="timeout">Timeout (s)</Label>
                <Input
                  id="timeout"
                  type="number"
                  value={formData.timeout}
                  onChange={(e) => handleInputChange('timeout', parseInt(e.target.value) || 3)}
                  min="1"
                  max="300"
                />
              </div>
              <div className="space-y-2">
                <Label htmlFor="memorySize">Memory Size (MB)</Label>
                <Input
                  id="memorySize"
                  type="number"
                  value={formData.memorySize}
                  onChange={(e) => handleInputChange('memorySize', parseInt(e.target.value) || 512)}
                  min="128"
                  max="10240"
                />
              </div>
            </div>

            <div className="space-y-2">
              <Label>Environment Variables</Label>
            {envRows.map((r, idx) => (
              <div key={idx} className="grid grid-cols-2 gap-2 mt-1">
                <Input
                  placeholder="KEY"
                  value={r.key}
                  onChange={(e) => {
                    const arr = [...envRows];
                    arr[idx] = { ...r, key: e.target.value };
                    setEnvRows(arr);
                  }}
                />
                {r.isSecret ? (
                  <select className="border rounded px-2 py-1 text-sm" value={r.secretName || ''} onChange={e=>{ const arr=[...envRows]; arr[idx]={...r, secretName:e.target.value}; setEnvRows(arr); }}>
                    <option value="">Select secret…</option>
                    {secrets.map(n=> <option key={n} value={n}>{n}</option>)}
                  </select>
                ) : (
                  <Input
                    placeholder="value"
                    value={r.value}
                    onChange={(e) => {
                      const arr = [...envRows];
                      arr[idx] = { ...r, value: e.target.value };
                      setEnvRows(arr);
                    }}
                  />
                )}
                <div className="col-span-2 text-xs text-gray-600">
                  <label className="inline-flex items-center space-x-2">
                    <input type="checkbox" checked={!!r.isSecret} onChange={e=>{ const arr=[...envRows]; arr[idx]={...r, isSecret:e.target.checked}; if (!e.target.checked) arr[idx].secretName=undefined; setEnvRows(arr); }} />
                    <span>Use secret (masked)</span>
                  </label>
                </div>
              </div>
            ))}
              <div className="mt-2">
                <Button
                  variant="outline"
                  size="sm"
                  type="button"
                  onClick={() => setEnvRows([...envRows, { key: '', value: '' }])}
                >
                  Add variable
                </Button>
              </div>
            </div>

            <div className="space-y-2">
              <Label htmlFor="zipFile">Code Package *</Label>
              <div className="flex items-center space-x-2">
                <Input
                  id="zipFile"
                  type="file"
                  accept=".zip"
                  onChange={handleFileChange}
                  className="flex-1"
                  required
                />
                <Upload className="h-4 w-4 text-muted-foreground" />
              </div>
              <p className="text-sm text-muted-foreground">
                Upload a ZIP file containing your function code
              </p>
              {zipFile && (
                <p className="text-sm text-green-600">
                  Selected: {zipFile.name} ({(zipFile.size / 1024).toFixed(1)} KB)
                </p>
              )}
            </div>

            <div className="flex justify-end space-x-2">
              <Button
                type="button"
                variant="outline"
                onClick={() => navigate('/functions')}
              >
                Cancel
              </Button>
              <Button
                type="submit"
                disabled={createFunction.isPending || isUploading}
              >
                {createFunction.isPending || isUploading ? 'Creating...' : 'Create Function'}
              </Button>
            </div>
          </form>
        </CardContent>
      </Card>
    </div>
  );
}

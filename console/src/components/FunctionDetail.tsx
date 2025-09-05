import { useEffect, useState } from 'react';
import { useParams, Link } from 'react-router-dom';
import { ArrowLeft, Settings, Trash2 } from 'lucide-react';
import { Button } from './ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from './ui/card';
import { useFunction, useDeleteFunction } from '../hooks/useFunctions';
import { formatBytes, formatDate, getStateColor } from '../lib/utils';
import { useToast } from './ui/use-toast';
import { InvokeEditor } from './InvokeEditor';
import { api } from '../lib/api';
import { useWarmPoolSummary } from '../hooks/useWarmPool';
import { UpdateFunctionModal } from './UpdateFunctionModal';

export function FunctionDetail() {
  const { name } = useParams<{ name: string }>();
  const { data: functionData, isLoading, error } = useFunction(name || '');
  const deleteFunction = useDeleteFunction();
  const { toast } = useToast();
  const [showUpdate, setShowUpdate] = useState(false);
  // Call all hooks unconditionally to preserve hook order across renders
  const fnName = name || '';
  const { data: pool, isLoading: poolLoading } = useWarmPoolSummary(fnName);
  const [gwMappings, setGwMappings] = useState<{ path: string; method?: string }[]>([]);

  useEffect(() => {
    let mounted = true;
    (async () => {
      try {
        const res = await api.listApiRoutes();
        const matched = res.routes
          .filter(r => r.function_name === fnName)
          .map(r => ({ path: r.path, method: r.method || 'ANY' }));
        if (mounted) setGwMappings(matched);
      } catch {}
    })();
    return () => { mounted = false; };
  }, [fnName]);

  const handleDelete = async () => {
    if (!name) return;
    
    if (window.confirm(`Are you sure you want to delete function "${name}"?`)) {
      try {
        await deleteFunction.mutateAsync(name);
        toast({
          title: "Function deleted",
          description: `Function "${name}" has been deleted successfully.`,
        });
        // Navigate back to functions list
        window.location.href = '/functions';
      } catch (error) {
        toast({
          title: "Error",
          description: `Failed to delete function: ${error instanceof Error ? error.message : 'Unknown error'}`,
          variant: "destructive",
        });
      }
    }
  };

  if (isLoading) {
    return (
      <div className="flex items-center justify-center h-64">
        <div className="text-lg">Loading function...</div>
      </div>
    );
  }

  if (error || !functionData) {
    return (
      <div className="flex items-center justify-center h-64">
        <div className="text-lg text-red-600">
          Error loading function: {error instanceof Error ? error.message : 'Function not found'}
        </div>
      </div>
    );
  }

  const func = functionData;

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div className="flex items-center space-x-4">
          <Button variant="outline" size="sm" asChild>
            <Link to="/functions">
              <ArrowLeft className="mr-2 h-4 w-4" />
              Back to Functions
            </Link>
          </Button>
          <div>
            <h1 className="text-3xl font-bold tracking-tight">{func.function_name}</h1>
            <p className="text-muted-foreground">
              Function details and test invocation
            </p>
          </div>
        </div>
        <div className="flex items-center space-x-2">
          <Button variant="outline" size="sm" onClick={()=>setShowUpdate(true)}>
            <Settings className="mr-2 h-4 w-4" />
            Settings
          </Button>
          <Button
            variant="outline"
            size="sm"
            onClick={handleDelete}
            disabled={deleteFunction.isPending}
          >
            <Trash2 className="mr-2 h-4 w-4" />
            Delete
          </Button>
        </div>
      </div>
      <UpdateFunctionModal
        open={showUpdate}
        onClose={()=>setShowUpdate(false)}
        name={func.function_name}
        initial={{ handler: func.handler, timeout: func.timeout, memory_size: func.memory_size, environment: func.environment, description: func.description }}
      />

      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        {/* Function Information */}
        <div className="space-y-6">
          <Card>
            <CardHeader>
              <CardTitle>Function Information</CardTitle>
              <CardDescription>
                Basic configuration and metadata
              </CardDescription>
            </CardHeader>
            <CardContent className="space-y-4">
              <div className="grid grid-cols-2 gap-4">
                <div>
                  <label className="text-sm font-medium text-muted-foreground">Status</label>
                  <div className="mt-1">
                    <span className={`inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium ${getStateColor(func.state)}`}>
                      {func.state}
                    </span>
                  </div>
                </div>
                <div>
                  <label className="text-sm font-medium text-muted-foreground">Runtime</label>
                  <p className="mt-1 text-sm">{func.runtime}</p>
                </div>
                <div>
                  <label className="text-sm font-medium text-muted-foreground">Handler</label>
                  <p className="mt-1 text-sm font-mono">{func.handler}</p>
                </div>
                <div>
                  <label className="text-sm font-medium text-muted-foreground">Version</label>
                  <p className="mt-1 text-sm">{func.version}</p>
                </div>
                <div>
                  <label className="text-sm font-medium text-muted-foreground">Timeout</label>
                  <p className="mt-1 text-sm">{func.timeout}s</p>
                </div>
                <div>
                  <label className="text-sm font-medium text-muted-foreground">Memory</label>
                  <p className="mt-1 text-sm">{func.memory_size}MB</p>
                </div>
                <div>
                  <label className="text-sm font-medium text-muted-foreground">Code Size</label>
                  <p className="mt-1 text-sm">{formatBytes(func.code_size)}</p>
                </div>
                <div>
                  <label className="text-sm font-medium text-muted-foreground">Last Modified</label>
                  <p className="mt-1 text-sm">{formatDate(func.last_modified)}</p>
                </div>
              </div>
              
              {func.description && (
                <div>
                  <label className="text-sm font-medium text-muted-foreground">Description</label>
                  <p className="mt-1 text-sm">{func.description}</p>
                </div>
              )}

              {Object.keys(func.environment).length > 0 && (
                <div>
                  <label className="text-sm font-medium text-muted-foreground">Environment Variables</label>
                  <div className="mt-1 space-y-1">
                    {Object.entries(func.environment).map(([key, value]) => (
                      <div key={key} className="flex items-center space-x-2 text-sm font-mono">
                        <span className="text-blue-600">{key}</span>
                        <span>=</span>
                        <span className="text-green-600">"{value}"</span>
                      </div>
                    ))}
                  </div>
                </div>
              )}
            </CardContent>
          </Card>

          <Card>
            <CardHeader>
              <CardTitle>Warm Pool</CardTitle>
              <CardDescription>
                Live container state for this function
              </CardDescription>
            </CardHeader>
            <CardContent className="space-y-3">
              {poolLoading ? (
                <div className="text-sm text-muted-foreground">Loading pool...</div>
              ) : pool ? (
                <>
                  <div className="flex space-x-4 text-sm">
                    <div>Total: <span className="font-medium">{pool.total}</span></div>
                    <div>WarmIdle: <span className="font-medium">{pool.warm_idle}</span></div>
                    <div>Active: <span className="font-medium">{pool.active}</span></div>
                    <div>Stopped: <span className="font-medium">{pool.stopped}</span></div>
                  </div>
                  <div className="max-h-48 overflow-auto border rounded">
                    <table className="w-full text-sm">
                      <thead>
                        <tr className="bg-gray-50 text-left">
                          <th className="px-2 py-1">Container ID</th>
                          <th className="px-2 py-1">State</th>
                          <th className="px-2 py-1">Idle For</th>
                        </tr>
                      </thead>
                      <tbody>
                        {pool.entries.slice(0, 50).map((e) => (
                          <tr key={e.container_id} className="border-t">
                            <td className="px-2 py-1 font-mono truncate max-w-[16rem]">{e.container_id}</td>
                            <td className="px-2 py-1">{e.state}</td>
                            <td className="px-2 py-1">{Math.round(e.idle_for_ms/1000)}s</td>
                          </tr>
                        ))}
                      </tbody>
                    </table>
                  </div>
                </>
              ) : (
                <div className="text-sm text-muted-foreground">No pool data</div>
              )}
            </CardContent>
          </Card>

          <Card>
            <CardHeader>
              <CardTitle>API Gateway Mappings</CardTitle>
              <CardDescription>
                Paths routed to this function
              </CardDescription>
            </CardHeader>
            <CardContent className="space-y-4">
              {gwMappings.length === 0 ? (
                <div className="text-sm text-muted-foreground">No API Gateway routes target this function.</div>
              ) : (
                <div className="border rounded overflow-auto">
                  <table className="w-full text-sm">
                    <thead>
                      <tr className="bg-gray-50 text-left">
                        <th className="px-2 py-1">Path</th>
                        <th className="px-2 py-1">Method</th>
                      </tr>
                    </thead>
                    <tbody>
                      {gwMappings.map((m, idx) => (
                        <tr key={idx} className="border-t">
                          <td className="px-2 py-1 font-mono">{m.path}</td>
                          <td className="px-2 py-1">{m.method}</td>
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
              <CardTitle>Code Information</CardTitle>
              <CardDescription>
                Code package details
              </CardDescription>
            </CardHeader>
            <CardContent className="space-y-4">
              <div className="grid grid-cols-2 gap-4">
                <div>
                  <label className="text-sm font-medium text-muted-foreground">SHA256</label>
                  <p className="mt-1 text-sm font-mono break-all">{func.code_sha256}</p>
                </div>
                <div>
                  <label className="text-sm font-medium text-muted-foreground">Function ID</label>
                  <p className="mt-1 text-sm font-mono">{func.function_id}</p>
                </div>
              </div>
            </CardContent>
          </Card>
        </div>

        {/* Test Invocation */}
        <div>
          <InvokeEditor functionName={func.function_name} />
        </div>
      </div>
    </div>
  );
}

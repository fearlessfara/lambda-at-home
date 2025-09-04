
import { CheckCircle, XCircle, Loader2 } from 'lucide-react';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from './ui/card';
import { useHealthCheck } from '../hooks/useFunctions';

export function HealthCheck() {
  const { data: healthData, isLoading, error } = useHealthCheck();

  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-3xl font-bold tracking-tight">Health Check</h1>
        <p className="text-muted-foreground">
          Monitor the health status of your Lambda@Home service
        </p>
      </div>

      <Card>
        <CardHeader>
          <CardTitle className="flex items-center">
            Service Status
          </CardTitle>
          <CardDescription>
            Current health status of the Lambda@Home API
          </CardDescription>
        </CardHeader>
        <CardContent>
          <div className="flex items-center space-x-4">
            {isLoading ? (
              <>
                <Loader2 className="h-6 w-6 animate-spin text-blue-500" />
                <div>
                  <p className="text-lg font-medium">Checking...</p>
                  <p className="text-sm text-muted-foreground">Verifying service health</p>
                </div>
              </>
            ) : error ? (
              <>
                <XCircle className="h-6 w-6 text-red-500" />
                <div>
                  <p className="text-lg font-medium text-red-600">Service Unavailable</p>
                  <p className="text-sm text-muted-foreground">
                    {error instanceof Error ? error.message : 'Unknown error'}
                  </p>
                </div>
              </>
            ) : (
              <>
                <CheckCircle className="h-6 w-6 text-green-500" />
                <div>
                  <p className="text-lg font-medium text-green-600">Service Healthy</p>
                  <p className="text-sm text-muted-foreground">
                    API is responding normally
                  </p>
                </div>
              </>
            )}
          </div>

          {healthData && (
            <div className="mt-4 p-3 bg-gray-50 rounded-md">
              <p className="text-sm font-mono">{healthData}</p>
            </div>
          )}
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>System Information</CardTitle>
          <CardDescription>
            Lambda@Home service details
          </CardDescription>
        </CardHeader>
        <CardContent>
          <div className="grid grid-cols-2 gap-4 text-sm">
            <div>
              <label className="font-medium text-muted-foreground">API Base URL</label>
              <p className="mt-1 font-mono">{import.meta.env.VITE_API_URL || 'http://localhost:9000'}</p>
            </div>
            <div>
              <label className="font-medium text-muted-foreground">Console Version</label>
              <p className="mt-1">v0.1.0</p>
            </div>
            <div>
              <label className="font-medium text-muted-foreground">Last Check</label>
              <p className="mt-1">{new Date().toLocaleString()}</p>
            </div>
            <div>
              <label className="font-medium text-muted-foreground">Status</label>
              <p className="mt-1">
                {isLoading ? 'Checking...' : error ? 'Unhealthy' : 'Healthy'}
              </p>
            </div>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}


import { CheckCircle, XCircle, Loader2, HardDrive, Cpu, MemoryStick, Container, Zap, Activity } from 'lucide-react';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from './ui/card';
import { useHealthCheck, useDockerStats, useLambdaServiceStats } from '../hooks/useFunctions';

export function HealthCheck() {
  const { data: healthData, isLoading, error } = useHealthCheck();
  const { data: dockerStats, isLoading: dockerStatsLoading, error: dockerStatsError } = useDockerStats();
  const { data: lambdaStats, isLoading: lambdaStatsLoading, error: lambdaStatsError } = useLambdaServiceStats();

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

      {/* Lambda Service Statistics */}
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center">
            <Zap className="h-5 w-5 mr-2" />
            Lambda Service Statistics
          </CardTitle>
          <CardDescription>
            Lambda functions and service performance metrics
          </CardDescription>
        </CardHeader>
        <CardContent>
          {lambdaStatsLoading ? (
            <div className="flex items-center space-x-4">
              <Loader2 className="h-6 w-6 animate-spin text-blue-500" />
              <div>
                <p className="text-lg font-medium">Loading Lambda Stats...</p>
                <p className="text-sm text-muted-foreground">Fetching function and performance information</p>
              </div>
            </div>
          ) : lambdaStatsError ? (
            <div className="flex items-center space-x-4">
              <XCircle className="h-6 w-6 text-red-500" />
              <div>
                <p className="text-lg font-medium text-red-600">Lambda Stats Unavailable</p>
                <p className="text-sm text-muted-foreground">
                  {lambdaStatsError instanceof Error ? lambdaStatsError.message : 'Failed to fetch Lambda statistics'}
                </p>
              </div>
            </div>
          ) : lambdaStats ? (
            <div className="space-y-6">
              {/* Function Overview */}
              <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
                <div className="flex items-center space-x-3 p-3 bg-gray-50 rounded-lg">
                  <Activity className="h-5 w-5 text-blue-500" />
                  <div>
                    <p className="text-sm font-medium text-muted-foreground">Total Functions</p>
                    <p className="text-2xl font-bold">{lambdaStats.total_functions}</p>
                    <p className="text-xs text-muted-foreground">
                      {lambdaStats.active_functions} active
                    </p>
                  </div>
                </div>
                <div className="flex items-center space-x-3 p-3 bg-gray-50 rounded-lg">
                  <MemoryStick className="h-5 w-5 text-green-500" />
                  <div>
                    <p className="text-sm font-medium text-muted-foreground">Total Memory</p>
                    <p className="text-2xl font-bold">{lambdaStats.total_memory_mb}MB</p>
                    <p className="text-xs text-muted-foreground">
                      {Math.round(lambdaStats.total_memory_mb / 1024)}GB allocated
                    </p>
                  </div>
                </div>
                <div className="flex items-center space-x-3 p-3 bg-gray-50 rounded-lg">
                  <Cpu className="h-5 w-5 text-purple-500" />
                  <div>
                    <p className="text-sm font-medium text-muted-foreground">CPU Cores</p>
                    <p className="text-2xl font-bold">{lambdaStats.total_cpu_cores}</p>
                    <p className="text-xs text-muted-foreground">
                      estimated allocation
                    </p>
                  </div>
                </div>
                <div className="flex items-center space-x-3 p-3 bg-gray-50 rounded-lg">
                  <Container className="h-5 w-5 text-orange-500" />
                  <div>
                    <p className="text-sm font-medium text-muted-foreground">Warm Containers</p>
                    <p className="text-2xl font-bold">{lambdaStats.warm_containers}</p>
                    <p className="text-xs text-muted-foreground">
                      {lambdaStats.active_containers} active, {lambdaStats.idle_containers} idle
                    </p>
                  </div>
                </div>
              </div>

              {/* Function States */}
              <div className="p-4 bg-gray-50 rounded-lg">
                <h4 className="font-medium mb-3">Function States</h4>
                <div className="grid grid-cols-2 md:grid-cols-4 gap-4 text-sm">
                  <div>
                    <p className="font-medium text-muted-foreground">Active</p>
                    <p className="text-lg font-bold text-green-600">{lambdaStats.active_functions}</p>
                  </div>
                  <div>
                    <p className="font-medium text-muted-foreground">Stopped</p>
                    <p className="text-lg font-bold text-gray-600">{lambdaStats.stopped_functions}</p>
                  </div>
                  <div>
                    <p className="font-medium text-muted-foreground">Failed</p>
                    <p className="text-lg font-bold text-red-600">{lambdaStats.failed_functions}</p>
                  </div>
                  <div>
                    <p className="font-medium text-muted-foreground">Total</p>
                    <p className="text-lg font-bold">{lambdaStats.total_functions}</p>
                  </div>
                </div>
              </div>

              {/* Performance Metrics (24h) */}
              <div className="p-4 bg-gray-50 rounded-lg">
                <h4 className="font-medium mb-3">Performance Metrics (Last 24h)</h4>
                <div className="grid grid-cols-2 md:grid-cols-4 gap-4 text-sm">
                  <div>
                    <p className="font-medium text-muted-foreground">Total Invocations</p>
                    <p className="text-lg font-bold">{lambdaStats.total_invocations_24h}</p>
                  </div>
                  <div>
                    <p className="font-medium text-muted-foreground">Successful</p>
                    <p className="text-lg font-bold text-green-600">{lambdaStats.successful_invocations_24h}</p>
                  </div>
                  <div>
                    <p className="font-medium text-muted-foreground">Failed</p>
                    <p className="text-lg font-bold text-red-600">{lambdaStats.failed_invocations_24h}</p>
                  </div>
                  <div>
                    <p className="font-medium text-muted-foreground">Success Rate</p>
                    <p className="text-lg font-bold">
                      {lambdaStats.total_invocations_24h > 0 
                        ? Math.round((lambdaStats.successful_invocations_24h / lambdaStats.total_invocations_24h) * 100)
                        : 0}%
                    </p>
                  </div>
                </div>
              </div>

              {/* Duration Statistics */}
              {lambdaStats.total_invocations_24h > 0 && (
                <div className="p-4 bg-gray-50 rounded-lg">
                  <h4 className="font-medium mb-3">Duration Statistics (Last 24h)</h4>
                  <div className="grid grid-cols-2 md:grid-cols-3 gap-4 text-sm">
                    <div>
                      <p className="font-medium text-muted-foreground">Average</p>
                      <p className="text-lg font-bold">{lambdaStats.avg_duration_ms.toFixed(1)}ms</p>
                    </div>
                    <div>
                      <p className="font-medium text-muted-foreground">Maximum</p>
                      <p className="text-lg font-bold">{lambdaStats.max_duration_ms.toFixed(1)}ms</p>
                    </div>
                    <div>
                      <p className="font-medium text-muted-foreground">Minimum</p>
                      <p className="text-lg font-bold">{lambdaStats.min_duration_ms.toFixed(1)}ms</p>
                    </div>
                  </div>
                </div>
              )}
            </div>
          ) : null}
        </CardContent>
      </Card>

      {/* Docker Statistics */}
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center">
            <Container className="h-5 w-5 mr-2" />
            Docker Statistics
          </CardTitle>
          <CardDescription>
            Docker daemon and container statistics
          </CardDescription>
        </CardHeader>
        <CardContent>
          {dockerStatsLoading ? (
            <div className="flex items-center space-x-4">
              <Loader2 className="h-6 w-6 animate-spin text-blue-500" />
              <div>
                <p className="text-lg font-medium">Loading Docker Stats...</p>
                <p className="text-sm text-muted-foreground">Fetching container and system information</p>
              </div>
            </div>
          ) : dockerStatsError ? (
            <div className="flex items-center space-x-4">
              <XCircle className="h-6 w-6 text-red-500" />
              <div>
                <p className="text-lg font-medium text-red-600">Docker Stats Unavailable</p>
                <p className="text-sm text-muted-foreground">
                  {dockerStatsError instanceof Error ? dockerStatsError.message : 'Failed to fetch Docker statistics'}
                </p>
              </div>
            </div>
          ) : dockerStats ? (
            <div className="space-y-6">
              {/* System Overview */}
              <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
                <div className="flex items-center space-x-3 p-3 bg-gray-50 rounded-lg">
                  <Container className="h-5 w-5 text-blue-500" />
                  <div>
                    <p className="text-sm font-medium text-muted-foreground">Containers</p>
                    <p className="text-2xl font-bold">{dockerStats.system_info.containers}</p>
                    <p className="text-xs text-muted-foreground">
                      {dockerStats.system_info.containers_running} running
                    </p>
                  </div>
                </div>
                <div className="flex items-center space-x-3 p-3 bg-gray-50 rounded-lg">
                  <HardDrive className="h-5 w-5 text-green-500" />
                  <div>
                    <p className="text-sm font-medium text-muted-foreground">Images</p>
                    <p className="text-2xl font-bold">{dockerStats.system_info.images}</p>
                  </div>
                </div>
                <div className="flex items-center space-x-3 p-3 bg-gray-50 rounded-lg">
                  <Cpu className="h-5 w-5 text-purple-500" />
                  <div>
                    <p className="text-sm font-medium text-muted-foreground">CPU Cores</p>
                    <p className="text-2xl font-bold">{dockerStats.system_info.cpu_count}</p>
                  </div>
                </div>
                <div className="flex items-center space-x-3 p-3 bg-gray-50 rounded-lg">
                  <MemoryStick className="h-5 w-5 text-orange-500" />
                  <div>
                    <p className="text-sm font-medium text-muted-foreground">Total Memory</p>
                    <p className="text-2xl font-bold">
                      {Math.round(dockerStats.system_info.memory_total / (1024 * 1024 * 1024))}GB
                    </p>
                  </div>
                </div>
              </div>

              {/* Docker Version & System Info */}
              <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
                <div className="p-4 bg-gray-50 rounded-lg">
                  <h4 className="font-medium mb-2">Docker Version</h4>
                  <div className="space-y-1 text-sm">
                    <p><span className="font-medium">Version:</span> {dockerStats.version.version}</p>
                    <p><span className="font-medium">API Version:</span> {dockerStats.version.api_version}</p>
                    <p><span className="font-medium">OS:</span> {dockerStats.version.os}</p>
                    <p><span className="font-medium">Architecture:</span> {dockerStats.version.arch}</p>
                  </div>
                </div>
                <div className="p-4 bg-gray-50 rounded-lg">
                  <h4 className="font-medium mb-2">System Info</h4>
                  <div className="space-y-1 text-sm">
                    <p><span className="font-medium">OS:</span> {dockerStats.system_info.operating_system}</p>
                    <p><span className="font-medium">Kernel:</span> {dockerStats.system_info.kernel_version}</p>
                    <p><span className="font-medium">Storage Driver:</span> {dockerStats.system_info.storage_driver}</p>
                    <p><span className="font-medium">Cgroup Driver:</span> {dockerStats.system_info.cgroup_driver}</p>
                  </div>
                </div>
              </div>

              {/* Disk Usage */}
              <div className="p-4 bg-gray-50 rounded-lg">
                <h4 className="font-medium mb-3">Disk Usage</h4>
                <div className="grid grid-cols-2 md:grid-cols-4 gap-4 text-sm">
                  <div>
                    <p className="font-medium text-muted-foreground">Layers Size</p>
                    <p className="text-lg font-bold">
                      {Math.round(dockerStats.disk_usage.layers_size / (1024 * 1024 * 1024) * 100) / 100}GB
                    </p>
                  </div>
                  <div>
                    <p className="font-medium text-muted-foreground">Images</p>
                    <p className="text-lg font-bold">{dockerStats.disk_usage.images.length}</p>
                  </div>
                  <div>
                    <p className="font-medium text-muted-foreground">Containers</p>
                    <p className="text-lg font-bold">{dockerStats.disk_usage.containers.length}</p>
                  </div>
                  <div>
                    <p className="font-medium text-muted-foreground">Volumes</p>
                    <p className="text-lg font-bold">{dockerStats.disk_usage.volumes.length}</p>
                  </div>
                </div>
              </div>

              {/* Cache Statistics */}
              <div className="p-4 bg-gray-50 rounded-lg">
                <h4 className="font-medium mb-3">Cache Performance</h4>
                <div className="grid grid-cols-2 md:grid-cols-4 gap-4 text-sm">
                  <div>
                    <p className="font-medium text-muted-foreground">Functions Cache</p>
                    <p className="text-lg font-bold">{dockerStats.cache_stats.functions.hit_rate.toFixed(1)}%</p>
                    <p className="text-xs text-muted-foreground">
                      {dockerStats.cache_stats.functions.hits} hits / {dockerStats.cache_stats.functions.misses} misses
                    </p>
                  </div>
                  <div>
                    <p className="font-medium text-muted-foreground">Concurrency Cache</p>
                    <p className="text-lg font-bold">{dockerStats.cache_stats.concurrency.hit_rate.toFixed(1)}%</p>
                    <p className="text-xs text-muted-foreground">
                      {dockerStats.cache_stats.concurrency.hits} hits / {dockerStats.cache_stats.concurrency.misses} misses
                    </p>
                  </div>
                  <div>
                    <p className="font-medium text-muted-foreground">Environment Cache</p>
                    <p className="text-lg font-bold">{dockerStats.cache_stats.env_vars.hit_rate.toFixed(1)}%</p>
                    <p className="text-xs text-muted-foreground">
                      {dockerStats.cache_stats.env_vars.hits} hits / {dockerStats.cache_stats.env_vars.misses} misses
                    </p>
                  </div>
                  <div>
                    <p className="font-medium text-muted-foreground">Secrets Cache</p>
                    <p className="text-lg font-bold">{dockerStats.cache_stats.secrets.hit_rate.toFixed(1)}%</p>
                    <p className="text-xs text-muted-foreground">
                      {dockerStats.cache_stats.secrets.hits} hits / {dockerStats.cache_stats.secrets.misses} misses
                    </p>
                  </div>
                </div>
              </div>
            </div>
          ) : null}
        </CardContent>
      </Card>
    </div>
  );
}

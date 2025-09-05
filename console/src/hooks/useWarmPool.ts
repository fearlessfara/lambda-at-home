import { useQuery } from '@tanstack/react-query';
import { api } from '../lib/api';

export function useWarmPoolSummary(name: string) {
  return useQuery({
    queryKey: ['warm-pool', name],
    queryFn: () => api.warmPoolSummary(name),
    enabled: !!name,
    refetchInterval: 5000,
  });
}


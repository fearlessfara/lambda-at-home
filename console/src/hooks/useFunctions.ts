import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { api } from '../lib/api';
import { CreateFunctionRequest } from '../types/api';

export function useFunctions() {
  return useQuery({
    queryKey: ['functions'],
    queryFn: () => api.listFunctions(),
    refetchInterval: 5000, // Refetch every 5 seconds
  });
}

export function useFunction(name: string) {
  return useQuery({
    queryKey: ['function', name],
    queryFn: () => api.getFunction(name),
    enabled: !!name,
  });
}

export function useCreateFunction() {
  const queryClient = useQueryClient();
  
  return useMutation({
    mutationFn: (data: CreateFunctionRequest) => api.createFunction(data),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['functions'] });
    },
  });
}

export function useDeleteFunction() {
  const queryClient = useQueryClient();
  
  return useMutation({
    mutationFn: (name: string) => api.deleteFunction(name),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['functions'] });
    },
  });
}

export function useInvokeFunction() {
  return useMutation({
    mutationFn: ({ name, payload, logType }: { 
      name: string; 
      payload: any; 
      logType?: 'None' | 'Tail' 
    }) => api.invokeFunction(name, payload, logType),
  });
}

export function useHealthCheck() {
  return useQuery({
    queryKey: ['health'],
    queryFn: () => api.healthCheck(),
    refetchInterval: 30000, // Check every 30 seconds
    retry: 3,
  });
}

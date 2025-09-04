
import { BrowserRouter as Router, Routes, Route, Navigate } from 'react-router-dom';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { Toaster } from './components/ui/toaster';
import { Layout } from './components/Layout';
import { FunctionList } from './components/FunctionList';
import { CreateFunction } from './components/CreateFunction';
import { FunctionDetail } from './components/FunctionDetail';
import { HealthCheck } from './components/HealthCheck';

// Create a client
const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      retry: 1,
      refetchOnWindowFocus: false,
    },
  },
});

function App() {
  return (
    <QueryClientProvider client={queryClient}>
      <Router>
        <Layout>
          <Routes>
            <Route path="/" element={<Navigate to="/functions" replace />} />
            <Route path="/functions" element={<FunctionList />} />
            <Route path="/functions/create" element={<CreateFunction />} />
            <Route path="/functions/:name" element={<FunctionDetail />} />
            <Route path="/health" element={<HealthCheck />} />
          </Routes>
        </Layout>
        <Toaster />
      </Router>
    </QueryClientProvider>
  );
}

export default App;

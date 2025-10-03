
import { Link } from 'react-router-dom';
import { Plus, Trash2, Play, ChevronLeft, ChevronRight } from 'lucide-react';
import { Button } from './ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from './ui/card';
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from './ui/table';
import { useFunctions, useDeleteFunction } from '../hooks/useFunctions';
import { formatBytes, formatDate, getStateColor } from '../lib/utils';
import { useToast } from './ui/use-toast';
import { useState } from 'react';

export function FunctionList() {
  const [currentPage, setCurrentPage] = useState(0);
  const [pageSize] = useState(20);
  
  const { data: functionsData, isLoading, error } = useFunctions({
    marker: currentPage > 0 ? (currentPage * pageSize).toString() : undefined,
    maxItems: pageSize,
  });
  const deleteFunction = useDeleteFunction();
  const { toast } = useToast();

  const handleDelete = async (name: string) => {
    if (window.confirm(`Are you sure you want to delete function "${name}"?`)) {
      try {
        await deleteFunction.mutateAsync(name);
        toast({
          title: "Function deleted",
          description: `Function "${name}" has been deleted successfully.`,
        });
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
        <div className="text-lg">Loading functions...</div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="flex items-center justify-center h-64">
        <div className="text-lg text-red-600">
          Error loading functions: {error instanceof Error ? error.message : 'Unknown error'}
        </div>
      </div>
    );
  }

  const functions = functionsData?.functions || [];
  const totalCount = functionsData?.total_count || 0;
  const hasNextPage = !!functionsData?.next_marker;
  const hasPreviousPage = currentPage > 0;
  const totalPages = Math.ceil(totalCount / pageSize);

  const handleNextPage = () => {
    if (hasNextPage) {
      setCurrentPage(prev => prev + 1);
    }
  };

  const handlePreviousPage = () => {
    if (hasPreviousPage) {
      setCurrentPage(prev => prev - 1);
    }
  };

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-3xl font-bold tracking-tight">Functions</h1>
          <p className="text-muted-foreground">
            Manage your Lambda@Home functions
            {totalCount > 0 && (
              <span className="ml-2 text-sm">
                ({totalCount} total)
              </span>
            )}
          </p>
        </div>
        <Button asChild>
          <Link to="/functions/create">
            <Plus className="mr-2 h-4 w-4" />
            Create Function
          </Link>
        </Button>
      </div>

      {functions.length === 0 ? (
        <Card>
          <CardHeader>
            <CardTitle>No functions found</CardTitle>
            <CardDescription>
              Get started by creating your first function.
            </CardDescription>
          </CardHeader>
          <CardContent>
            <Button asChild>
              <Link to="/functions/create">
                <Plus className="mr-2 h-4 w-4" />
                Create Function
              </Link>
            </Button>
          </CardContent>
        </Card>
      ) : (
        <Card>
          <CardHeader>
            <CardTitle>
              Functions ({functions.length} of {totalCount})
            </CardTitle>
            <CardDescription>
              A list of all your Lambda@Home functions
              {totalPages > 1 && (
                <span className="ml-2 text-sm">
                  (Page {currentPage + 1} of {totalPages})
                </span>
              )}
            </CardDescription>
          </CardHeader>
          <CardContent>
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>Name</TableHead>
                  <TableHead>Runtime</TableHead>
                  <TableHead>Status</TableHead>
                  <TableHead>Last Modified</TableHead>
                  <TableHead>Code Size</TableHead>
                  <TableHead>Actions</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {functions.map((func) => (
                  <TableRow key={func.function_id}>
                    <TableCell className="font-medium">
                      <Link 
                        to={`/functions/${func.function_name}`}
                        className="text-blue-600 hover:text-blue-800 hover:underline"
                      >
                        {func.function_name}
                      </Link>
                    </TableCell>
                    <TableCell>{func.runtime}</TableCell>
                    <TableCell>
                      <span className={`inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium ${getStateColor(func.state)}`}>
                        {func.state}
                      </span>
                    </TableCell>
                    <TableCell>{formatDate(func.last_modified)}</TableCell>
                    <TableCell>{formatBytes(func.code_size)}</TableCell>
                    <TableCell>
                      <div className="flex items-center space-x-2">
                        <Button
                          variant="outline"
                          size="sm"
                          asChild
                        >
                          <Link to={`/functions/${func.function_name}`}>
                            <Play className="h-4 w-4" />
                          </Link>
                        </Button>
                        <Button
                          variant="outline"
                          size="sm"
                          onClick={() => handleDelete(func.function_name)}
                          disabled={deleteFunction.isPending}
                        >
                          <Trash2 className="h-4 w-4" />
                        </Button>
                      </div>
                    </TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
            
            {/* Pagination Controls */}
            {totalPages > 1 && (
              <div className="flex items-center justify-between mt-4 pt-4 border-t">
                <div className="text-sm text-muted-foreground">
                  Showing {currentPage * pageSize + 1} to {Math.min((currentPage + 1) * pageSize, totalCount)} of {totalCount} functions
                </div>
                <div className="flex items-center space-x-2">
                  <Button
                    variant="outline"
                    size="sm"
                    onClick={handlePreviousPage}
                    disabled={!hasPreviousPage}
                  >
                    <ChevronLeft className="h-4 w-4" />
                    Previous
                  </Button>
                  <Button
                    variant="outline"
                    size="sm"
                    onClick={handleNextPage}
                    disabled={!hasNextPage}
                  >
                    Next
                    <ChevronRight className="h-4 w-4" />
                  </Button>
                </div>
              </div>
            )}
          </CardContent>
        </Card>
      )}
    </div>
  );
}

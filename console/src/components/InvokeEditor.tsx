import { useState } from 'react';
import { Play, Copy, Check } from 'lucide-react';
import { Button } from './ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from './ui/card';
import { useInvokeFunction } from '../hooks/useFunctions';
import { useToast } from './ui/use-toast';
import Editor from '@monaco-editor/react';

interface InvokeEditorProps {
  functionName: string;
}

export function InvokeEditor({ functionName }: InvokeEditorProps) {
  const [payload, setPayload] = useState('{\n  "message": "Hello, Lambda@Home!"\n}');
  const [result, setResult] = useState<any>(null);
  const [isValidJson, setIsValidJson] = useState(true);
  const [copied, setCopied] = useState(false);
  
  const invokeFunction = useInvokeFunction();
  const { toast } = useToast();

  const validateJson = (jsonString: string) => {
    try {
      JSON.parse(jsonString);
      setIsValidJson(true);
      return true;
    } catch {
      setIsValidJson(false);
      return false;
    }
  };

  const handlePayloadChange = (value: string | undefined) => {
    const newValue = value || '';
    setPayload(newValue);
    validateJson(newValue);
  };

  const handleInvoke = async () => {
    if (!isValidJson) {
      toast({
        title: "Invalid JSON",
        description: "Please fix the JSON syntax errors before invoking.",
        variant: "destructive",
      });
      return;
    }

    try {
      const parsedPayload = JSON.parse(payload);
      const response = await invokeFunction.mutateAsync({
        name: functionName,
        payload: parsedPayload,
        logType: 'Tail',
      });

      setResult({
        response: response.response,
        statusCode: response.statusCode,
        executedVersion: response.executedVersion,
        functionError: response.functionError,
        logResult: response.logResult,
        duration: response.duration,
      });

      toast({
        title: "Function invoked",
        description: `Function executed successfully in ${response.duration}ms`,
      });
    } catch (error) {
      toast({
        title: "Invocation failed",
        description: `Failed to invoke function: ${error instanceof Error ? error.message : 'Unknown error'}`,
        variant: "destructive",
      });
      
      setResult({
        error: error instanceof Error ? error.message : 'Unknown error',
        duration: 0,
      });
    }
  };

  const copyToClipboard = async (text: string) => {
    try {
      await navigator.clipboard.writeText(text);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
      toast({
        title: "Copied to clipboard",
        description: "Result has been copied to your clipboard.",
      });
    } catch (error) {
      toast({
        title: "Copy failed",
        description: "Failed to copy to clipboard.",
        variant: "destructive",
      });
    }
  };

  const formatResult = (result: any) => {
    if (result.error) {
      return JSON.stringify({ error: result.error }, null, 2);
    }
    return JSON.stringify(result.response, null, 2);
  };

  return (
    <div className="space-y-6">
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center">
            <Play className="mr-2 h-5 w-5" />
            Test Function
          </CardTitle>
          <CardDescription>
            Invoke your function with a test payload
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          <div>
            <label className="text-sm font-medium">Test Event</label>
            <div className="mt-2 border rounded-md overflow-hidden">
              <Editor
                height="200px"
                defaultLanguage="json"
                value={payload}
                onChange={handlePayloadChange}
                options={{
                  minimap: { enabled: false },
                  scrollBeyondLastLine: false,
                  fontSize: 14,
                  lineNumbers: 'on',
                  wordWrap: 'on',
                  automaticLayout: true,
                }}
                theme="vs-light"
              />
            </div>
            {!isValidJson && (
              <p className="mt-1 text-sm text-red-600">Invalid JSON syntax</p>
            )}
          </div>

          <Button
            onClick={handleInvoke}
            disabled={invokeFunction.isPending || !isValidJson}
            className="w-full"
          >
            {invokeFunction.isPending ? 'Invoking...' : 'Invoke Function'}
          </Button>
        </CardContent>
      </Card>

      {result && (
        <Card>
          <CardHeader>
            <CardTitle className="flex items-center justify-between">
              <span>Execution Result</span>
              <Button
                variant="outline"
                size="sm"
                onClick={() => copyToClipboard(formatResult(result))}
              >
                {copied ? <Check className="h-4 w-4" /> : <Copy className="h-4 w-4" />}
                {copied ? 'Copied' : 'Copy'}
              </Button>
            </CardTitle>
            <CardDescription>
              Function execution details and response
            </CardDescription>
          </CardHeader>
          <CardContent className="space-y-4">
            <div className="grid grid-cols-2 gap-4 text-sm">
              <div>
                <label className="font-medium text-muted-foreground">Duration</label>
                <p className="mt-1">{result.duration}ms</p>
              </div>
              {result.statusCode && (
                <div>
                  <label className="font-medium text-muted-foreground">Status Code</label>
                  <p className="mt-1">{result.statusCode}</p>
                </div>
              )}
              {result.executedVersion && (
                <div>
                  <label className="font-medium text-muted-foreground">Executed Version</label>
                  <p className="mt-1">{result.executedVersion}</p>
                </div>
              )}
              {result.functionError && (
                <div>
                  <label className="font-medium text-muted-foreground">Function Error</label>
                  <p className="mt-1 text-red-600">{result.functionError}</p>
                </div>
              )}
            </div>

            {result.logResult && (
              <div>
                <label className="text-sm font-medium text-muted-foreground">Log Output</label>
                <div className="mt-2 p-3 bg-gray-50 rounded-md">
                  <pre className="text-sm font-mono whitespace-pre-wrap break-words">
                    {atob(result.logResult)}
                  </pre>
                </div>
              </div>
            )}

            <div>
              <label className="text-sm font-medium text-muted-foreground">Response</label>
              <div className="mt-2 border rounded-md overflow-hidden">
                <Editor
                  height="200px"
                  defaultLanguage="json"
                  value={formatResult(result)}
                  options={{
                    minimap: { enabled: false },
                    scrollBeyondLastLine: false,
                    fontSize: 14,
                    lineNumbers: 'on',
                    wordWrap: 'on',
                    automaticLayout: true,
                    readOnly: true,
                  }}
                  theme="vs-light"
                />
              </div>
            </div>
          </CardContent>
        </Card>
      )}
    </div>
  );
}

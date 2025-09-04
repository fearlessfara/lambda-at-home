# Lambda@Home Console

A modern React-based web console for managing and testing Lambda@Home functions, similar to the AWS Lambda console.

## Features

- **Function Management**: Create, view, and delete Lambda@Home functions
- **Code Upload**: Upload ZIP files containing your function code
- **Runtime Selection**: Support for Node.js 18, Python 3.11, and Rust runtimes
- **Test Invocation**: Test your functions with custom JSON payloads
- **Real-time Monitoring**: View function status, execution results, and logs
- **Health Check**: Monitor the health status of your Lambda@Home service

## Tech Stack

- **React 18** with TypeScript
- **Vite** for fast development and building
- **Tailwind CSS** for styling
- **shadcn/ui** for UI components
- **React Query** for data fetching and caching
- **React Router** for navigation
- **Monaco Editor** for JSON editing

## Getting Started

### Prerequisites

- Node.js 18+ 
- npm or yarn
- Lambda@Home service running on `http://localhost:9000`

### Installation

1. Navigate to the console directory:
   ```bash
   cd console
   ```

2. Install dependencies:
   ```bash
   npm install
   ```

3. Start the development server:
   ```bash
   npm run dev
   ```

4. Open your browser and navigate to `http://localhost:3000`

### Environment Configuration

Create a `.env` file in the console directory to configure the API URL:

```env
VITE_API_URL=http://localhost:9000
```

## Available Scripts

- `npm run dev` - Start the development server
- `npm run build` - Build the application for production
- `npm run preview` - Preview the production build
- `npm run lint` - Run ESLint

## Project Structure

```
console/
├── src/
│   ├── components/          # React components
│   │   ├── ui/             # shadcn/ui components
│   │   ├── FunctionList.tsx
│   │   ├── CreateFunction.tsx
│   │   ├── FunctionDetail.tsx
│   │   ├── InvokeEditor.tsx
│   │   ├── Layout.tsx
│   │   └── HealthCheck.tsx
│   ├── hooks/              # Custom React hooks
│   │   └── useFunctions.ts
│   ├── lib/                # Utility functions
│   │   ├── api.ts
│   │   └── utils.ts
│   ├── types/              # TypeScript type definitions
│   │   └── api.ts
│   ├── App.tsx
│   ├── main.tsx
│   └── index.css
├── public/
├── package.json
├── vite.config.ts
├── tailwind.config.js
└── README.md
```

## Usage

### Creating a Function

1. Click "Create Function" on the functions list page
2. Fill in the function details:
   - **Function Name**: Unique identifier for your function
   - **Runtime**: Choose from nodejs18.x, python3.11, or rust
   - **Handler**: Entry point for your function (e.g., `index.handler`)
   - **Description**: Optional description
   - **Timeout**: Maximum execution time in seconds
   - **Memory Size**: Memory allocation in MB
3. Upload a ZIP file containing your function code
4. Click "Create Function"

### Testing a Function

1. Navigate to a function's detail page
2. In the "Test Function" section, modify the JSON payload
3. Click "Invoke Function" to test your function
4. View the execution result, duration, and any log output

### Supported Runtimes

- **Node.js 18**: Use `index.handler` as the handler format
- **Python 3.11**: Use `lambda_function.lambda_handler` as the handler format  
- **Rust**: Use `main` as the handler format

## API Integration

The console integrates with the Lambda@Home User API endpoints:

- `GET /2015-03-31/functions` - List functions
- `POST /2015-03-31/functions` - Create function
- `GET /2015-03-31/functions/:name` - Get function details
- `DELETE /2015-03-31/functions/:name` - Delete function
- `POST /2015-03-31/functions/:name/invocations` - Invoke function
- `GET /healthz` - Health check

## Development

### Adding New Features

1. Create new components in `src/components/`
2. Add API functions to `src/lib/api.ts`
3. Create custom hooks in `src/hooks/`
4. Update types in `src/types/api.ts`

### Styling

The project uses Tailwind CSS with shadcn/ui components. To add new styles:

1. Use Tailwind utility classes
2. Create custom components using shadcn/ui primitives
3. Add custom CSS in `src/index.css` if needed

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Test thoroughly
5. Submit a pull request

## License

This project is part of the Lambda@Home monorepo and follows the same license terms.

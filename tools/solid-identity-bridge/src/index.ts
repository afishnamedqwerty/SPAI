/**
 * Solid Identity Bridge - TypeScript subprocess for SPAI
 * Handles Solid-OIDC, WebID, and Pod operations via JSON-RPC over stdio
 */

import * as readline from 'readline';
import { handleRequest } from './handlers';

// JSON-RPC request interface
interface JsonRpcRequest {
  jsonrpc: '2.0';
  id: number | string | null;
  method: string;
  params: any;
}

// JSON-RPC response interface
interface JsonRpcResponse {
  jsonrpc: '2.0';
  id: number | string | null;
  result?: any;
  error?: {
    code: number;
    message: string;
    data?: any;
  };
}

// Set up readline interface for stdio communication
const rl = readline.createInterface({
  input: process.stdin,
  output: process.stdout,
  terminal: false,
});

// Log to stderr (stdout is reserved for JSON-RPC responses)
const log = (message: string) => {
  process.stderr.write(`[solid-bridge] ${message}\n`);
};

log('Solid Identity Bridge started');

// Handle incoming JSON-RPC requests
rl.on('line', async (line: string) => {
  try {
    const request: JsonRpcRequest = JSON.parse(line);

    log(`Received request: ${request.method}`);

    // Handle the request
    const result = await handleRequest(request);

    // Send successful response
    const response: JsonRpcResponse = {
      jsonrpc: '2.0',
      id: request.id,
      result,
    };

    // Write to stdout (single line JSON)
    console.log(JSON.stringify(response));
  } catch (error: any) {
    // Log error to stderr
    log(`Error processing request: ${error.message}`);

    // Try to send error response
    try {
      const request: JsonRpcRequest = JSON.parse(line);
      const errorResponse: JsonRpcResponse = {
        jsonrpc: '2.0',
        id: request?.id || null,
        error: {
          code: -32603,
          message: error.message || 'Internal error',
          data: error.stack,
        },
      };
      console.log(JSON.stringify(errorResponse));
    } catch {
      // If we can't even parse the request, send generic error
      const errorResponse: JsonRpcResponse = {
        jsonrpc: '2.0',
        id: null,
        error: {
          code: -32700,
          message: 'Parse error',
        },
      };
      console.log(JSON.stringify(errorResponse));
    }
  }
});

// Handle process termination
process.on('SIGTERM', () => {
  log('Received SIGTERM, shutting down');
  process.exit(0);
});

process.on('SIGINT', () => {
  log('Received SIGINT, shutting down');
  process.exit(0);
});

// Handle uncaught errors
process.on('uncaughtException', (error) => {
  log(`Uncaught exception: ${error.message}`);
  process.exit(1);
});

process.on('unhandledRejection', (reason) => {
  log(`Unhandled rejection: ${reason}`);
  process.exit(1);
});

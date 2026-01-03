"use strict";
/**
 * Solid Identity Bridge - TypeScript subprocess for SPAI
 * Handles Solid-OIDC, WebID, and Pod operations via JSON-RPC over stdio
 */
var __createBinding = (this && this.__createBinding) || (Object.create ? (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    var desc = Object.getOwnPropertyDescriptor(m, k);
    if (!desc || ("get" in desc ? !m.__esModule : desc.writable || desc.configurable)) {
      desc = { enumerable: true, get: function() { return m[k]; } };
    }
    Object.defineProperty(o, k2, desc);
}) : (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    o[k2] = m[k];
}));
var __setModuleDefault = (this && this.__setModuleDefault) || (Object.create ? (function(o, v) {
    Object.defineProperty(o, "default", { enumerable: true, value: v });
}) : function(o, v) {
    o["default"] = v;
});
var __importStar = (this && this.__importStar) || (function () {
    var ownKeys = function(o) {
        ownKeys = Object.getOwnPropertyNames || function (o) {
            var ar = [];
            for (var k in o) if (Object.prototype.hasOwnProperty.call(o, k)) ar[ar.length] = k;
            return ar;
        };
        return ownKeys(o);
    };
    return function (mod) {
        if (mod && mod.__esModule) return mod;
        var result = {};
        if (mod != null) for (var k = ownKeys(mod), i = 0; i < k.length; i++) if (k[i] !== "default") __createBinding(result, mod, k[i]);
        __setModuleDefault(result, mod);
        return result;
    };
})();
Object.defineProperty(exports, "__esModule", { value: true });
const readline = __importStar(require("readline"));
const handlers_1 = require("./handlers");
// Set up readline interface for stdio communication
const rl = readline.createInterface({
    input: process.stdin,
    output: process.stdout,
    terminal: false,
});
// Log to stderr (stdout is reserved for JSON-RPC responses)
const log = (message) => {
    process.stderr.write(`[solid-bridge] ${message}\n`);
};
log('Solid Identity Bridge started');
// Handle incoming JSON-RPC requests
rl.on('line', async (line) => {
    try {
        const request = JSON.parse(line);
        log(`Received request: ${request.method}`);
        // Handle the request
        const result = await (0, handlers_1.handleRequest)(request);
        // Send successful response
        const response = {
            jsonrpc: '2.0',
            id: request.id,
            result,
        };
        // Write to stdout (single line JSON)
        console.log(JSON.stringify(response));
    }
    catch (error) {
        // Log error to stderr
        log(`Error processing request: ${error.message}`);
        // Try to send error response
        try {
            const request = JSON.parse(line);
            const errorResponse = {
                jsonrpc: '2.0',
                id: request?.id || null,
                error: {
                    code: -32603,
                    message: error.message || 'Internal error',
                    data: error.stack,
                },
            };
            console.log(JSON.stringify(errorResponse));
        }
        catch {
            // If we can't even parse the request, send generic error
            const errorResponse = {
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
//# sourceMappingURL=index.js.map
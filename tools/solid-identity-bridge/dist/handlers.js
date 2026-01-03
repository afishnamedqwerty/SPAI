"use strict";
/**
 * Request handlers for Solid operations
 */
Object.defineProperty(exports, "__esModule", { value: true });
exports.handleRequest = handleRequest;
const solid_client_1 = require("@inrupt/solid-client");
const solid_client_authn_node_1 = require("@inrupt/solid-client-authn-node");
const vocab_common_rdf_1 = require("@inrupt/vocab-common-rdf");
// Solid vocabulary namespace
const SOLID = {
    oidcIssuer: 'http://www.w3.org/ns/solid/terms#oidcIssuer',
};
// Global session instance
const session = new solid_client_authn_node_1.Session();
async function handleRequest(request) {
    const { method, params } = request;
    switch (method) {
        case 'fetchProfile':
            return await fetchWebIdProfile(params.webid);
        case 'authenticate':
            return await performOidcAuth(params);
        case 'fetchResource':
            return await fetchSolidResource(params.url, params.contentType);
        case 'updateResource':
            return await updateSolidResource(params.url, params.data, params.contentType);
        case 'executeSparql':
            return await executeSparqlQuery(params.endpoint, params.query);
        case 'logout':
            return await performLogout();
        case 'getSessionInfo':
            return await getSessionInfo();
        default:
            throw new Error(`Unknown method: ${method}`);
    }
}
/**
 * Fetch WebID profile document
 */
async function fetchWebIdProfile(webid) {
    try {
        const profileDataset = await (0, solid_client_1.getSolidDataset)(webid);
        const profile = (0, solid_client_1.getThing)(profileDataset, webid);
        if (!profile) {
            throw new Error(`Could not find profile at ${webid}`);
        }
        return {
            webid,
            name: (0, solid_client_1.getStringNoLocale)(profile, vocab_common_rdf_1.FOAF.name) || null,
            oidcIssuer: (0, solid_client_1.getUrl)(profile, SOLID.oidcIssuer) || null,
            storage: (0, solid_client_1.getUrl)(profile, 'http://www.w3.org/ns/pim/space#storage') || null,
            inbox: (0, solid_client_1.getUrl)(profile, 'http://www.w3.org/ns/ldp#inbox') || null,
        };
    }
    catch (error) {
        throw new Error(`Failed to fetch WebID profile: ${error.message}`);
    }
}
/**
 * Perform Solid-OIDC authentication
 */
async function performOidcAuth(params) {
    try {
        // For now, we'll use client credentials flow
        // In a real implementation, this would handle the full OIDC flow
        await session.login({
            oidcIssuer: params.issuer,
            clientId: params.clientId,
            redirectUrl: params.redirectUri,
        });
        const info = session.info;
        return {
            isLoggedIn: info.isLoggedIn,
            webId: info.webId,
            sessionId: info.sessionId,
            // Note: Access token handling would be done with DPoP in Rust
            expiresAt: Date.now() + (3600 * 1000), // 1 hour default
        };
    }
    catch (error) {
        throw new Error(`Authentication failed: ${error.message}`);
    }
}
/**
 * Fetch a Solid resource
 */
async function fetchSolidResource(url, contentType) {
    try {
        const dataset = await (0, solid_client_1.getSolidDataset)(url, {
            fetch: session.fetch,
        });
        // For now, return a simple serialization
        // In production, we'd use a proper RDF serializer
        return {
            url,
            contentType: contentType || 'text/turtle',
            content: JSON.stringify(dataset),
        };
    }
    catch (error) {
        throw new Error(`Failed to fetch resource: ${error.message}`);
    }
}
/**
 * Update a Solid resource
 */
async function updateSolidResource(url, data, contentType) {
    try {
        // Create or update dataset
        let dataset = (0, solid_client_1.createSolidDataset)();
        // In production, parse the data according to contentType
        // For now, we'll assume it's already in the right format
        await (0, solid_client_1.saveSolidDatasetAt)(url, dataset, {
            fetch: session.fetch,
        });
        return {
            success: true,
            url,
        };
    }
    catch (error) {
        throw new Error(`Failed to update resource: ${error.message}`);
    }
}
/**
 * Execute SPARQL query
 */
async function executeSparqlQuery(endpoint, query) {
    try {
        // This would require a SPARQL client
        // For now, return placeholder
        return {
            bindings: [],
            query,
            endpoint,
        };
    }
    catch (error) {
        throw new Error(`SPARQL query failed: ${error.message}`);
    }
}
/**
 * Logout from current session
 */
async function performLogout() {
    try {
        await session.logout();
        return {
            success: true,
        };
    }
    catch (error) {
        throw new Error(`Logout failed: ${error.message}`);
    }
}
/**
 * Get current session information
 */
async function getSessionInfo() {
    const info = session.info;
    return {
        isLoggedIn: info.isLoggedIn,
        webId: info.webId || null,
        sessionId: info.sessionId,
    };
}
//# sourceMappingURL=handlers.js.map
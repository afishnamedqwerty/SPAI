/**
 * Request handlers for Solid operations
 */

import {
  getSolidDataset,
  getThing,
  getStringNoLocale,
  getUrl,
  saveSolidDatasetAt,
  createSolidDataset,
  setThing,
  createThing,
  addStringNoLocale,
} from '@inrupt/solid-client';
import { Session } from '@inrupt/solid-client-authn-node';
import { FOAF, VCARD } from '@inrupt/vocab-common-rdf';

// Solid vocabulary namespace
const SOLID = {
  oidcIssuer: 'http://www.w3.org/ns/solid/terms#oidcIssuer',
};

// Global session instance
const session = new Session();

export async function handleRequest(request: any): Promise<any> {
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
async function fetchWebIdProfile(webid: string): Promise<any> {
  try {
    const profileDataset = await getSolidDataset(webid);
    const profile = getThing(profileDataset, webid);

    if (!profile) {
      throw new Error(`Could not find profile at ${webid}`);
    }

    return {
      webid,
      name: getStringNoLocale(profile, FOAF.name) || null,
      oidcIssuer: getUrl(profile, SOLID.oidcIssuer) || null,
      storage: getUrl(profile, 'http://www.w3.org/ns/pim/space#storage') || null,
      inbox: getUrl(profile, 'http://www.w3.org/ns/ldp#inbox') || null,
    };
  } catch (error: any) {
    throw new Error(`Failed to fetch WebID profile: ${error.message}`);
  }
}

/**
 * Perform Solid-OIDC authentication
 */
async function performOidcAuth(params: {
  issuer: string;
  clientId: string;
  redirectUri: string;
  dpopProof?: string;
}): Promise<any> {
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
  } catch (error: any) {
    throw new Error(`Authentication failed: ${error.message}`);
  }
}

/**
 * Fetch a Solid resource
 */
async function fetchSolidResource(
  url: string,
  contentType?: string
): Promise<any> {
  try {
    const dataset = await getSolidDataset(url, {
      fetch: session.fetch,
    });

    // For now, return a simple serialization
    // In production, we'd use a proper RDF serializer
    return {
      url,
      contentType: contentType || 'text/turtle',
      content: JSON.stringify(dataset),
    };
  } catch (error: any) {
    throw new Error(`Failed to fetch resource: ${error.message}`);
  }
}

/**
 * Update a Solid resource
 */
async function updateSolidResource(
  url: string,
  data: any,
  contentType?: string
): Promise<any> {
  try {
    // Create or update dataset
    let dataset = createSolidDataset();

    // In production, parse the data according to contentType
    // For now, we'll assume it's already in the right format

    await saveSolidDatasetAt(url, dataset, {
      fetch: session.fetch,
    });

    return {
      success: true,
      url,
    };
  } catch (error: any) {
    throw new Error(`Failed to update resource: ${error.message}`);
  }
}

/**
 * Execute SPARQL query
 */
async function executeSparqlQuery(
  endpoint: string,
  query: string
): Promise<any> {
  try {
    // This would require a SPARQL client
    // For now, return placeholder
    return {
      bindings: [],
      query,
      endpoint,
    };
  } catch (error: any) {
    throw new Error(`SPARQL query failed: ${error.message}`);
  }
}

/**
 * Logout from current session
 */
async function performLogout(): Promise<any> {
  try {
    await session.logout();
    return {
      success: true,
    };
  } catch (error: any) {
    throw new Error(`Logout failed: ${error.message}`);
  }
}

/**
 * Get current session information
 */
async function getSessionInfo(): Promise<any> {
  const info = session.info;
  return {
    isLoggedIn: info.isLoggedIn,
    webId: info.webId || null,
    sessionId: info.sessionId,
  };
}

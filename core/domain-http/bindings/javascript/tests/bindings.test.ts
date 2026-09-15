import { DomainClient } from '@auki/domain-client';
import { expect, it, vi } from 'vitest';

const domainId = '00000000-0000-4000-8000-000000000001';
const upload = [{
    name: 'fixture',
    data_type: 'test',
    data: new TextEncoder().encode('fixture data'),
}];

// These tokens only exercise HTTP forwarding and expiry parsing. Every request
// is intercepted; the fixture does not verify backend signatures or policy.
function fixtureToken(type: string) {
    const encode = (value: object) => btoa(JSON.stringify(value))
        .replace(/=/g, '').replace(/\+/g, '-').replace(/\//g, '_');
    return `${encode({ alg: 'HS256', typ: 'JWT' })}.${encode({
        type, org: 'fixture-org', exp: Math.floor(Date.now() / 1000) + 3600,
    })}.fixture-signature`;
}

let fixtureId = 0;
function oidcFixture({
    serviceType = 'user-access', apiStatus = 200, domainStatus = 200, writeStatus = 403,
}: {
    serviceType?: 'app-access' | 'user-access';
    apiStatus?: number;
    domainStatus?: number;
    writeStatus?: number;
} = {}) {
    const api = 'https://api.invalid';
    const dds = 'https://dds.invalid';
    const server = `https://domain-${++fixtureId}.invalid`;
    const serviceToken = fixtureToken(serviceType);
    const domainToken = fixtureToken('domain-access');
    const requests: Request[] = [];
    const fetch = vi.spyOn(globalThis, 'fetch').mockImplementation(async (input, init) => {
        const request = new Request(input, init);
        requests.push(request);
        const reply = (body: BodyInit, status = 200, contentType = 'text/plain') => {
            const response = new Response(body, { status, headers: { 'Content-Type': contentType } });
            // reqwest reads the final URL, which a constructed Response lacks.
            Object.defineProperty(response, 'url', { value: request.url });
            return response;
        };
        const json = (value: unknown, status = 200) => reply(JSON.stringify(value), status, 'application/json');
        if (request.url === `${api}/service/domains-access-token`) {
            expect(request.method).toBe('POST');
            expect(request.headers.get('Authorization')).toBe('Bearer offline-oidc-token');
            return apiStatus === 200
                ? json({ access_token: serviceToken })
                : reply('invalid OIDC credential', apiStatus);
        }
        if (request.url === `${dds}/api/v1/domains/${domainId}/auth`) {
            expect(request.method).toBe('POST');
            expect(request.headers.get('Authorization')).toBe(`Bearer ${serviceToken}`);
            return domainStatus === 200 ? json({
                id: domainId, name: 'fixture', organization_id: 'fixture-org',
                domain_server_id: 'fixture-server',
                domain_server: {
                    id: 'fixture-server', organization_id: 'fixture-org',
                    name: 'fixture', url: server,
                },
                access_token: domainToken,
            }) : reply('Domain access denied', domainStatus);
        }
        if (request.url === `${server}/api/v1/info`) {
            expect(request.method).toBe('GET');
            return json({ upload: { request_max_bytes: 1024, multipart: { enabled: false } } });
        }
        if (request.url === `${server}/api/v1/domains/${domainId}/data/fixture?raw=true`) {
            expect(request.method).toBe('GET');
            expect(request.headers.get('Authorization')).toBe(`Bearer ${domainToken}`);
            return reply('fixture data');
        }
        if (request.url === `${server}/api/v1/domains/${domainId}/data`) {
            expect(request.method).toBe('POST');
            expect(request.headers.get('Authorization')).toBe(`Bearer ${domainToken}`);
            expect(await request.text()).toContain('fixture data');
            return writeStatus === 201 ? json({ data: [{
                id: 'created-data', domain_id: domainId, name: 'fixture', data_type: 'test',
                size: 12, created_at: '2026-01-01T00:00:00Z', updated_at: '2026-01-01T00:00:00Z',
            }] }, 201) : reply('data write denied', writeStatus);
        }
        throw new Error(`Unexpected request: ${request.method} ${request.url}`);
    });
    const base = new DomainClient(api, dds, 'bindings-test');
    const client = base.withOIDCAccessToken('offline-oidc-token');
    return {
        client, requests,
        close() {
            client.free();
            base.free();
            fetch.mockRestore();
        },
    };
}

it('initializes the WASM client and clones authentication without network access', () => {
    const fetch = vi.spyOn(globalThis, 'fetch').mockRejectedValue(new Error('Unexpected network request'));
    const client = new DomainClient('https://api.invalid', 'https://dds.invalid', 'bindings-test');
    let authenticated: DomainClient | undefined;
    try {
        authenticated = client.withOIDCAccessToken('offline-test-token');
        expect(authenticated).toBeInstanceOf(DomainClient);
        expect(authenticated).not.toBe(client);
        expect(fetch).not.toHaveBeenCalled();
    } finally {
        authenticated?.free();
        client.free();
        fetch.mockRestore();
    }
});

it.each(['app-access', 'user-access'] as const)(
    'propagates write denial after a successful OIDC read with %s', async (serviceType) => {
        const fixture = oidcFixture({ serviceType });
        try {
            const bytes = await fixture.client.downloadDomainDataById(domainId, 'fixture');
            expect(new TextDecoder().decode(bytes)).toBe('fixture data');
            await expect(fixture.client.uploadDomainData(domainId, upload)).rejects.toThrow(
                /403 Forbidden, error: Failed to create data\. data write denied/
            );
            expect(fixture.requests.filter(r => r.url.endsWith('/data'))).toHaveLength(1);
        } finally {
            fixture.close();
        }
    }
);

it('returns uploaded metadata when the OIDC writer is authorized', async () => {
    const fixture = oidcFixture({ writeStatus: 201 });
    try {
        const created = await fixture.client.uploadDomainData(domainId, upload);
        expect(created).toHaveLength(1);
        expect(created[0]).toMatchObject({ id: 'created-data', domain_id: domainId, size: 12 });
    } finally {
        fixture.close();
    }
});

it('stops before contacting storage when DDS denies the OIDC user access', async () => {
    const fixture = oidcFixture({ domainStatus: 403 });
    try {
        await expect(fixture.client.uploadDomainData(domainId, upload)).rejects.toThrow(
            /403 Forbidden, error: Failed to auth domain\. Domain access denied/
        );
        expect(fixture.requests.map(r => new URL(r.url).hostname)).toEqual(['api.invalid', 'dds.invalid']);
    } finally {
        fixture.close();
    }
});

it('stops at a rejected OIDC exchange without falling back to another identity', async () => {
    const fixture = oidcFixture({ apiStatus: 401 });
    try {
        await expect(fixture.client.downloadDomainDataById(domainId, 'fixture')).rejects.toThrow(
            /401 Unauthorized, error: Failed to get DDS access token\. invalid OIDC credential/
        );
        expect(fixture.requests).toHaveLength(1);
    } finally {
        fixture.close();
    }
});

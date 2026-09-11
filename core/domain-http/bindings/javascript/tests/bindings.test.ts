import { DomainClient } from '@auki/domain-client';
import { expect, it, vi } from 'vitest';

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

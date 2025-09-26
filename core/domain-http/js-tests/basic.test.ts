/// <reference types="vite/client" />

import { DownloadQuery, signInWithAppCredential, signInWithUserCredential, DomainClient, UploadDomainData, DomainData, DomainDataMetadata } from 'posemesh-domain-http';
import { describe, it, expect, beforeAll, afterAll } from 'vitest';

const loadConfig = () => {
    if (typeof process == 'undefined') {
        return import.meta.env;
    } else {
        return process.env;
    }
};

const config = loadConfig();

describe('Posemesh Domain HTTP', () => {
    describe('App Credential', async () => {
        let client: DomainClient;
        beforeAll(async () => {
            client = await signInWithAppCredential(
                config.API_URL,
                config.DDS_URL,
                config.CLIENT_ID,
                config.APP_KEY,
                config.APP_SECRET
            ) as DomainClient;
        });
        afterAll(() => {
            client.free();
        });

        it('should return error if app credential is invalid', async () => {
            await expect(async () => {
                await signInWithAppCredential(
                    config.API_URL,
                    config.DDS_URL,
                    config.CLIENT_ID,
                    "invalid-app-key",
                    "invalid-app-secret"
                );
            }).rejects.toThrow();
        });

        it('should download domain data with app credential', async () => {
            const data: DomainData[] = await client.downloadDomainData(config.DOMAIN_ID, {
                ids: [],
                name: null,
                data_type: "dmt_accel_csv"
            } as DownloadQuery);
            expect(data).toBeDefined();
            expect(data.length).greaterThan(0);

            for (const item of data) {
                expect(item.data.length).greaterThan(0);
                expect(item.metadata.data_type).toBe("dmt_accel_csv");
                expect(item.metadata.id).toBeDefined();
                expect(item.metadata.name).toBeDefined();
                expect(item.metadata.size).greaterThan(0);
                expect(item.metadata.created_at).toBeDefined();
                expect(item.metadata.updated_at).toBeDefined();
            }
        });

        it('should download a specific domain data by id', async () => {
            // Use a known data id for the test (from env or hardcoded for test)
            const dataId = "a84a36e5-312b-4f80-974a-06f5d19c1e16";
            const domainId = config.DOMAIN_ID;

            // Download the data by id
            const bytes = await client.downloadDomainDataById(domainId, dataId);

            // Check that we got a Uint8Array with length > 0
            expect(bytes).toBeDefined();
            // In browser, bytes is a Uint8Array; in node, it may be Buffer or Uint8Array
            // So check for .length and that it's > 0
            expect(bytes.length).greaterThan(0);
        });

        it('should download metadata for domain data', async () => {
            const query = {
                ids: [],
                name: null,
                data_type: "dmt_accel_csv"
            } as DownloadQuery;
            const metadata: DomainDataMetadata[] = await client.downloadDomainDataMetadata(config.DOMAIN_ID, query);

            expect(metadata).toBeDefined();
            expect(Array.isArray(metadata)).toBe(true);
            expect(metadata.length).toBeGreaterThan(0);

            for (const item of metadata) {
                expect(item.id).toBeDefined();
                expect(item.name).toBeDefined();
                expect(item.data_type).toBe("dmt_accel_csv");
                expect(item.size).toBeGreaterThan(0);
                expect(item.created_at).toBeDefined();
                expect(item.updated_at).toBeDefined();
            }
        });

        it('should download data as readablestream', async () => {
            const data: ReadableStream<DomainData> = await client.downloadDomainDataStream(config.DOMAIN_ID, {
                ids: [],
                name: null,
                data_type: "dmt_accel_csv"
            } as DownloadQuery);
            expect(data).toBeDefined();
            let count = 0;
            for await (const chunk of data) {
                count++;
                expect(chunk.metadata.data_type).toBe("dmt_accel_csv");
                expect(chunk.metadata.id).toBeDefined();
                expect(chunk.metadata.name).toBeDefined();
                expect(chunk.metadata.size).greaterThan(0);
                expect(chunk.metadata.created_at).toBeDefined();
                expect(chunk.metadata.updated_at).toBeDefined();
                expect(chunk.data.length).greaterThan(0);
            }
            expect(count).greaterThan(0);
        });

        it('should not upload domain data', async () => {
            const data = `{"test": "test updated2"}`;
            const dataBytes = new TextEncoder().encode(data);
            await expect(client.uploadDomainData(config.DOMAIN_ID, [{
                id: "a84a36e5-312b-4f80-974a-06f5d19c1e16",
                data: dataBytes,
            }])).rejects.toThrow(/Update failed with status: invalid domain access token/);
        });
    });

    describe('user credential', async () => {
        let client: DomainClient;
        beforeAll(async () => {
            client = await signInWithUserCredential(
                config.API_URL,
                config.DDS_URL,
                config.CLIENT_ID,
                config.POSEMESH_EMAIL,
                config.POSEMESH_PASSWORD,
                false
            );
        });
        afterAll(() => {
            client.free();
        });

        it('should sign in with user credential and download domain data', async () => {
            const dataList: DomainData[] = await client.downloadDomainData(config.DOMAIN_ID, {
                ids: [],
                name: null,
                data_type: "dmt_accel_csv"
            } as DownloadQuery);
            expect(Array.isArray(dataList)).toBe(true);
            expect(dataList.length).greaterThan(0);

            for (const item of dataList) {
                expect(item.metadata.id).toBeDefined();
                expect(item.metadata.name).toBeDefined();
                expect(item.metadata.data_type).toBe("dmt_accel_csv");
                expect(item.metadata.size).greaterThan(0);
                expect(item.metadata.created_at).toBeDefined();
                expect(item.metadata.updated_at).toBeDefined();
                expect(item.data.length).greaterThan(0);
            }
        });

        it('should download data as readablestream with user credential', async () => {
            const data: ReadableStream<DomainData> = await client.downloadDomainDataStream(config.DOMAIN_ID, {
                ids: ["a84a36e5-312b-4f80-974a-06f5d19c1e16"],
                name: null,
                data_type: null
            } as DownloadQuery);
            expect(data).toBeDefined();
            let count = 0;
            for await (const chunk of data) {
                count++;
                expect(chunk.data.length).greaterThan(0);
                expect(chunk.metadata.data_type).toBe("test");
                expect(chunk.metadata.id).toBe("a84a36e5-312b-4f80-974a-06f5d19c1e16");
                expect(chunk.metadata.name).toBeDefined();
                expect(chunk.metadata.size).greaterThan(0);
                expect(chunk.metadata.created_at).toBeDefined();
                expect(chunk.metadata.updated_at).toBeDefined();
            }
            expect(count).greaterThan(0);
        });

        it('should upload domain data with user credential', async () => {
            const data = `{"test": "test updated"}`;
            const dataBytes = new TextEncoder().encode(data);
            let res: DomainDataMetadata[] = await client.uploadDomainData(config.DOMAIN_ID, [{
                id: "a84a36e5-312b-4f80-974a-06f5d19c1e16",
                data: dataBytes,
            } as UploadDomainData, {
                name: "to be deleted - js test",
                data_type: "test",
                data: dataBytes,
            } as UploadDomainData]);
            expect(res.length).toBe(2);
            for (const item of res) {
                if (item.id !== "a84a36e5-312b-4f80-974a-06f5d19c1e16") {
                    await client.deleteDomainDataById(config.DOMAIN_ID, item.id);
                }
            }
        });

        it('should load domain metadata', async () => {
            const metadata: DomainDataMetadata[] = await client.downloadDomainDataMetadata(config.DOMAIN_ID, {
                ids: [],
                name: null,
                data_type: "test"
            } as DownloadQuery);
            expect(Array.isArray(metadata)).toBe(true);
            expect(metadata.length).toBeGreaterThan(0);

            for (const item of metadata) {
                expect(item).toHaveProperty("id");
                expect(item).toHaveProperty("name");
                expect(item).toHaveProperty("data_type");
                expect(item).toHaveProperty("size");
                expect(item).toHaveProperty("created_at");
                expect(item).toHaveProperty("updated_at");
            }
        });
    });

    describe('zitadel token', () => {
        const zitadelToken = config.AUTH_TEST_TOKEN;
        let client: DomainClient;
        let clientWithZitadelToken: DomainClient;
        beforeAll(() => {
            client = new DomainClient(config.API_URL, config.DDS_URL, config.CLIENT_ID);
            clientWithZitadelToken = client.withZitadelToken(zitadelToken);
        });
        afterAll(() => {
            clientWithZitadelToken.free();
            client.free();
        });
        
        it('should download domain data', async () => {
            const data: DomainData[] = await clientWithZitadelToken.downloadDomainData(config.DOMAIN_ID, {
                ids: [],
                name: null,
                data_type: "test"
            } as DownloadQuery);

            expect(data.length).toBeGreaterThan(0);
            for (const item of data) {
                expect(item.data.length).greaterThan(0);
                expect(item.metadata.data_type).toBe("test");
                expect(item.metadata.id).toBeDefined();
                expect(item.metadata.name).toBeDefined();
                expect(item.metadata.size).greaterThan(0);
            }
        });

        it('should download domain data metadata', async () => {
            const metadata: DomainDataMetadata[] = await clientWithZitadelToken.downloadDomainDataMetadata(config.DOMAIN_ID, {
                ids: [],
                name: null,
                data_type: "test"
            } as DownloadQuery);
            expect(metadata.length).toBeGreaterThan(0);
            for await (const chunk of metadata) {
                expect(chunk.size).greaterThan(0);
                expect(chunk.data_type).toBe("test");
                expect(chunk.id).toBeDefined();
                expect(chunk.name).toBeDefined();
            }
        });

        it('should download domain data stream', async () => {
            const data: ReadableStream<DomainData> = await clientWithZitadelToken.downloadDomainDataStream(config.DOMAIN_ID, {
                ids: [],
                name: null,
                data_type: "test"
            } as DownloadQuery);
            expect(data).toBeDefined();
            let count = 0;
            for await (const chunk of data) {
                count++;
                expect(chunk.data.length).greaterThan(0);
                expect(chunk.metadata.data_type).toBe("test");
                expect(chunk.metadata.id).toBeDefined();
                expect(chunk.metadata.name).toBeDefined();
                expect(chunk.metadata.size).greaterThan(0);
            }
            expect(count).greaterThan(0);
        });

        it('should upload domain data', async () => {
            const data = `{"zitadel": "token test"}`;
            const dataBytes = new TextEncoder().encode(data);
            let res: DomainDataMetadata[] = await clientWithZitadelToken.uploadDomainData(config.DOMAIN_ID, [{
                name: "zitadel token test",
                data_type: "test",
                data: dataBytes,
            } as UploadDomainData]);

            expect(res.length).toBe(1);
            expect(res[0].name).toBe("zitadel token test");
            expect(res[0].data_type).toBe("test");
            expect(res[0].size).toBe(dataBytes.length);
            expect(res[0].created_at).toBeDefined();
            expect(res[0].updated_at).toBeDefined();

            await clientWithZitadelToken.deleteDomainDataById(config.DOMAIN_ID, res[0].id);
        });

        it('should throw error if zitadel token is not valid', async () => {
            const invalidClient = client.withZitadelToken("ddddd");

            const data = `{"zitadel": "token test"}`;
            const dataBytes = new TextEncoder().encode(data);

            await expect(async () => {
                await invalidClient.uploadDomainData(config.DOMAIN_ID, [{
                    name: "zitadel token test",
                    data_type: "test",
                    data: dataBytes,
                } as UploadDomainData]);
            }).rejects.toThrow();

            invalidClient.free();
        });
    });

});

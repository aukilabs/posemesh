import init, { DomainCluster, RemoteDatastore, Query, DomainData, Metadata } from "./pkg/posemesh-domain.js";

export class UploadManager {
    constructor() {
        this.files = [];
        this.onFileSelect = () => {};
        this.onProgressUpdate = () => {};
        this.onUploadComplete = () => {};
        this.libp2pReady = false;
        this.datastore = null;
        this.producer = null;
        this.domainCluster = null;
    }

    async init(onFileSelect, onProgressUpdate, onUploadComplete) {
        this.onFileSelect = onFileSelect;
        this.onProgressUpdate = onProgressUpdate;
        this.onUploadComplete = onUploadComplete;
        
        try {
            await this.initializeLibp2p(); // Initialize libp2p on startup
        } catch (error) {
            console.error("Failed to initialize libp2p:", error);
        }
    }

    async initializeLibp2p() {
        try {
            console.log("initializing domain cluster");
            await init();
            console.log("wasm loaeded!");
            const domainCluster = new DomainCluster("/ip4/54.67.15.233/tcp/18803/ws/p2p/12D3KooWE7RYJVU3wCcXhzSSGdwm1fdiTiGsV9EJPnen47sSZMiL", "domain-browser-example", null, null);
            this.datastore = new RemoteDatastore(domainCluster);
            this.uploader = await this.datastore.produce("");
            this.libp2pReady = true;

            console.log("domain cluster is ready!");
        } catch (error) {
            console.error("Failed to initialize libp2p:", error);
        }
    }

    handleFiles(selectedFiles) {
        this.files = Array.from(selectedFiles);
        if (this.files.length > 0) {
            this.onFileSelect(this.files);
        }
    }

    async uploadFiles() {
        if (!this.libp2pReady) {
            console.error("Cannot upload: libp2p not initialized.");
            return;
        }

        for (const file of this.files) {
            try {
                const metadata = new Metadata(file.name, file.type, file.size, {}, "");
                const arrayBuffer = await file.arrayBuffer();
                const uint8Array = new Uint8Array(arrayBuffer);

                const contentPointer = uint8Array.buffer;
                const data = new DomainData("", metadata, contentPointer);
                const id = await this.uploader.push(data);
                console.log(`Pushed ${file.name} -> ${id}`);
            } catch (error) {
                console.error(`Failed to upload ${file.name}`, error);
            }
        }

        const intervalId = setInterval(async () => {
            let completed = await this.uploader.is_completed();
            if (completed) {
                console.log("Condition is true!");
                this.onUploadComplete();
                clearInterval(intervalId); // Stop checking once the condition is true
            } else {
                console.log("not done yet")
            }
        }, 1000);
    }

    async downloadFiles() {
        if (this.datastore != null) {
            const query = new Query([], [], [], null, null);
            console.log("Query created");

            const downloader = await this.datastore.consume("", query);
            console.log("Downloader initialized", downloader);
            return downloader;
        } else {
            console.error("Haven't initialized");
        }
    }
}

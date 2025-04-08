import init, { DomainCluster, RemoteDatastore, Query, DomainData, Metadata, reconstruction_job } from "posemesh-domain";
import * as proto from "./protobuf/task";
function getDataType(fileName) {
    const fileNameMap = {
        "Manifest.json": "dmt_manifest_json",
        "FeaturePoints.ply": "dmt_featurepoints_ply",
        "ARposes.csv": "dmt_arposes_csv",
        "PortalDetections.csv": "dmt_portal_detections_csv",
        "CameraIntrinsics.csv": "dmt_cameraintrinsics_csv",
        "Frames.csv": "dmt_frames_csv",
        "Gyro.csv": "dmt_gyro_csv",
        "Accel.csv": "dmt_accel_csv",
        "gyro_accel.csv": "dmt_gyroaccel_csv",
        "Frames.mp4": "dmt_recording_mp4"
    };

    return fileNameMap[fileName] || "";
}
function getCurrentTimeFormatted() {
    const now = new Date();
    
    const year = now.getFullYear();
    const month = String(now.getMonth() + 1).padStart(2, '0');  // +1 because months are 0-indexed
    const day = String(now.getDate()).padStart(2, '0');
    const hours = String(now.getHours()).padStart(2, '0');
    const minutes = String(now.getMinutes()).padStart(2, '0');
    const seconds = String(now.getSeconds()).padStart(2, '0');

    return `${year}-${month}-${day}_${hours}-${minutes}-${seconds}`;
}
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
        this.activeUploads = new Map();  // Track uploads: scan_name -> status
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
            const domainCluster = new DomainCluster("/ip4/127.0.0.1/udp/18801/webrtc-direct/certhash/uEiA2J2rDp90OcHCmtUn6PdGKWwxqkFpNeDx5ZT5Lla6AWA/p2p/12D3KooWDHaDQeuYeLM8b5zhNjqS7Pkh7KefqzCpDGpdwj5iE8pq", "domain-browser-example", null, null);

            this.domainCluster = domainCluster;
            this.datastore = new RemoteDatastore(domainCluster);
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

    async refine() {
        if (!this.libp2pReady) {
            console.error("Cannot refine: libp2p not initialized.");
            return;
        }
     
        this.uploader.close();
        this.uploader = null;
        let scans = this.activeUploads.keys();
        let scans_array = Array.from(scans);
        await reconstruction_job(this.domainCluster, scans_array, (taskBytes) => {
            const task = proto.task.Task.deserializeBinary(taskBytes); 
            console.log("reconstruction job", task.toObject());
        });
        this.activeUploads.clear();
        this.updateUploadsList();
    }

    async uploadFiles() {
        if (!this.libp2pReady) {
            console.error("Cannot upload: libp2p not initialized.");
            return;
        }

        if (this.uploader == null) {
            this.uploader = await this.datastore.produce("");
        }
        console.log("uploader initialized");

        const date = getCurrentTimeFormatted();
        const scan_name = prompt("Please enter a scan name:", date);
        if (!scan_name) {
            console.log("Upload cancelled - no scan name provided");
            return;
        }

        // Add scan to tracking with 'uploading' status
        this.activeUploads.set(scan_name, 'uploading');
        this.updateUploadsList();

        for (const file of this.files) {
            try {
                let data_type = getDataType(file.name);
                const metadata = new Metadata(data_type+"_"+scan_name, data_type, file.size, {}, "");
                const arrayBuffer = await file.arrayBuffer();
                const uint8Array = new Uint8Array(arrayBuffer);

                const data = new DomainData("", metadata, uint8Array);
                const id = await this.uploader.push(data);
                console.log(`Pushed ${file.name}: ${file.size} bytes -> ${id}`);
            } catch (error) {
                console.error(`Failed to upload ${file.name}`, error);
                this.activeUploads.set(scan_name, 'failed');
                this.updateUploadsList();
                return;
            }
        }

        const intervalId = setInterval(async () => {
            let completed = await this.uploader.is_completed();
            if (completed) {
                console.log("Upload complete!");
                this.activeUploads.set(scan_name, 'completed');
                this.updateUploadsList();
                this.onUploadComplete();
                clearInterval(intervalId);
            } else {
                console.log("not done yet");
            }
        }, 1000);
    }

    updateUploadsList() {
        const uploadsDiv = document.getElementById('uploads-list');
        uploadsDiv.innerHTML = '';

        for (const [scan_name, status] of this.activeUploads) {
            const uploadEntry = document.createElement('div');
            uploadEntry.className = 'flex items-center bg-gray-50 rounded-lg p-3 shadow-sm mb-2';
            
            const statusColors = {
                'uploading': 'bg-yellow-400',
                'completed': 'bg-green-500',
                'failed': 'bg-red-500'
            };

            uploadEntry.innerHTML = `
                <div class="flex items-center w-full">
                    <div class="w-2 h-2 rounded-full ${statusColors[status]} mr-2"></div>
                    <span class="text-sm font-medium text-gray-700 flex-grow">${scan_name}</span>
                </div>
            `;
            uploadsDiv.appendChild(uploadEntry);
        }
    }

    async downloadFiles() {
        if (this.datastore != null) {
            const query = new Query([], [], [], null, null);
            console.log("Query created");

            const downloader = await this.datastore.consume("", query);
            console.log("Downloader initialized");
            return downloader;
        } else {
            console.error("Haven't initialized");
        }
    }
}

// Initialize the application
async function initializeApp() {
    console.log("initializing app");
    // Get DOM elements
    const uploadManager = new UploadManager();

    // Get DOM elements
    const dropZone = document.getElementById("dropZone");
    const fileInput = document.getElementById("fileInput");
    const uploadBtn = document.getElementById("uploadBtn");
    const finishBtn = document.getElementById("finishBtn");
    const progressLabel = document.getElementById("progressLabel");
    const downloadBtn = document.getElementById("downloadBtn");
    const fileMetadata = document.getElementById("fileMetadata");

    // Set up drag and drop events
    dropZone.addEventListener("dragover", (e) => {
        e.preventDefault();
        dropZone.classList.add("bg-gray-200");
    });

    dropZone.addEventListener("dragleave", () => 
        dropZone.classList.remove("bg-gray-200")
    );

    dropZone.addEventListener("drop", (e) => {
        e.preventDefault();
        dropZone.classList.remove("bg-gray-200");
        uploadManager.handleFiles(e.dataTransfer.files);
    });

    // Add click event listener to dropZone
    dropZone.addEventListener("click", (e) => {
        // Only trigger if clicking directly on dropZone, not on fileInput
        if (e.target === dropZone) {
            fileInput.click();
        }
    });

    // Set up other event listeners
    fileInput.addEventListener("change", (e) => 
        uploadManager.handleFiles(e.target.files)
    );
    uploadBtn.addEventListener("click", () => uploadManager.uploadFiles());
    finishBtn.addEventListener("click", finishUpload);

    // Initialize upload manager
    await uploadManager.init(
        (files) => {
            uploadBtn.disabled = false;
            progressLabel.innerText = `${files.length} file(s) selected`;
        },
        (message) => {
            progressLabel.innerText = message;
        },
        () => {
            progressLabel.innerText = "All files uploaded!";
            uploadBtn.disabled = true;
            finishBtn.disabled = false;
        }
    );

    async function finishUpload() {
        fileInput.value = "";
        uploadBtn.disabled = true;
        finishBtn.disabled = true;
        progressLabel.innerText = "Triggering reconstruction...";
        await uploadManager.refine();
        progressLabel.innerText = "Reconstruction submitted!";
    }

    // Set up download functionality
    downloadBtn.addEventListener("click", async () => {
        const downloading = await uploadManager.downloadFiles();
        while(true) {
            const file = await downloading.next();
            if (!file) break;
            const metadata = file.metadata;
        
            const p = document.createElement("p");
            p.textContent = `${metadata.name} ${metadata.size} bytes`;
            console.log("file", metadata);
            
            fileMetadata.appendChild(p);
        }
    });

    // // Set up job monitoring
    // const fn = jobBytes => {
    //     console.log("received")
    //     const job = proto.task.Job.deserialize(jobBytes);
    //     updateJobTable(job);
    // }

    // setInterval(() => {
    //     uploadManager.domainCluster.monitor(fn);
    // }, 5000);
}

// Job table update functions
function updateJobTable(job) {
    const table = document.getElementById("jobsTable").getElementsByTagName("tbody")[0];
    let row = document.getElementById(`job-${job.id}`);

    if (!row) {
        row = table.insertRow();
        row.id = `job-${job.id}`;
        row.insertCell(0).textContent = job.id;
        row.insertCell(1).textContent = job.name;
        row.insertCell(2).appendChild(createTaskTable(job.tasks));
    } else {
        row.cells[2].replaceChild(createTaskTable(job.tasks), row.cells[2].firstChild);
    }
}

function createTaskTable(tasks) {
    const taskTable = document.createElement("table");
    taskTable.border = "1";

    const thead = taskTable.createTHead();
    const headerRow = thead.insertRow();
    ["Name", "Status", "Sender", "Receiver"].forEach((title) => {
        const th = document.createElement("th");
        th.textContent = title;
        headerRow.appendChild(th);
    });

    const tbody = taskTable.createTBody();
    tasks.forEach((task) => {
        const row = tbody.insertRow();
        row.insertCell(0).textContent = task.name;
        row.insertCell(1).textContent = task.status;
        row.insertCell(2).textContent = task.sender;
        row.insertCell(3).textContent = task.receiver;
    });

    return taskTable;
}

// Initialize the application when the DOM is loaded
document.addEventListener('DOMContentLoaded', initializeApp);

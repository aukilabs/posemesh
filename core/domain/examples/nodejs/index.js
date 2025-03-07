const domain = require("posemesh-domain");
const { Config, posemeshNetworkingContextCreate, DomainCluster, RemoteDatastore, Query, DomainData, Metadata } = domain;

async function main() {
    const cfg = new Config(
        "/ip4/127.0.0.1/udp/18800/quic-v1/p2p/12D3KooWDHaDQeuYeLM8b5zhNjqS7Pkh7KefqzCpDGpdwj5iE8pq",
        "",
        ""
    );

    const libp2pInstance = posemeshNetworkingContextCreate(cfg);
    console.log("Libp2p instance initialized");

    // const domainCluster = new DomainCluster("12D3KooWDHaDQeuYeLM8b5zhNjqS7Pkh7KefqzCpDGpdwj5iE8pq", libp2pInstance);
    // console.log("Domain cluster created");

    // const datastore = new RemoteDatastore(domainCluster, libp2pInstance);
    // console.log("Remote datastore initialized");

    const query = new Query([], null, null);
    console.log("Query created");

    const downloader = await datastore.consume("", query);
    console.log("Downloader initialized");

    // for await (let datum of downloader) {
    //     console.log(datum.metadata);
    // }
}

main().catch(console.error);

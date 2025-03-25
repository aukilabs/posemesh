const domain = require("posemesh-domain");
const { DomainCluster, RemoteDatastore, Query, DomainData, Metadata } = domain;

async function main() {

    const domainCluster = new DomainCluster("/ip4/54.67.15.233/tcp/18803/ws/p2p/12D3KooWE7RYJVU3wCcXhzSSGdwm1fdiTiGsV9EJPnen47sSZMiL", "domain-browser-example", null, "./volume/pkey");

    const datastore = new RemoteDatastore(domainCluster);

    const query = new Query([], null, null);
    console.log("Query created");

    const downloader = await datastore.consume("", query);
    console.log("Downloader initialized");
}

main().catch(console.error);

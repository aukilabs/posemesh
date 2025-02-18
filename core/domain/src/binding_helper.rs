use networking::context::Context;

use crate::{cluster::DomainCluster, datastore::remote::RemoteDatastore};

pub(crate) fn init_r_remote_storage(cluster: *mut DomainCluster, peer: *mut Context) -> RemoteDatastore {
    unsafe {
        // Ensure the pointers are not null
        assert!(!cluster.is_null(), "init_r_remote_storage(): cluster is null");
        assert!(!peer.is_null(), "init_r_remote_storage(): peer is null");

        // Copy the values without consuming the pointers
        let cluster_copy = (*cluster).clone();
        let peer_copy = (*peer).clone();

        RemoteDatastore::new(cluster_copy, peer_copy)
    }
}

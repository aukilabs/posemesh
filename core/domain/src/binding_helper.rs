use crate::{cluster::DomainCluster, datastore::remote::RemoteDatastore};

pub(crate) fn init_r_remote_storage(cluster: *mut DomainCluster) -> RemoteDatastore {
    unsafe {
        // Ensure the pointers are not null
        assert!(!cluster.is_null(), "init_r_remote_storage(): cluster is null");

        // Copy the values without consuming the pointers
        let cluster_copy = (*cluster).clone();

        RemoteDatastore::new(cluster_copy)
    }
}

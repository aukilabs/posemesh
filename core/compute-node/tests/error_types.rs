use posemesh_compute_node::errors::*;

const MOCK_CAPABILITY: &str = "/posemesh/mock/v1";

#[test]
fn errors_are_constructible_and_display() {
    let e: DmsClientError = DmsClientError::Http("400 bad".into());
    let _t: &dyn std::error::Error = &e;
    assert!(format!("{}", e).contains("http error"));

    let e: ExecutorError = ExecutorError::NoRunner(MOCK_CAPABILITY.into());
    let _t: &dyn std::error::Error = &e;
    assert!(format!("{}", e).contains("No runner") || format!("{}", e).contains("no runner"));

    let e: TokenManagerError = TokenManagerError::Rotation("oops".into());
    let _t: &dyn std::error::Error = &e;
    assert!(format!("{}", e).contains("rotation"));

    let e: StorageError = StorageError::Unauthorized;
    let _t: &dyn std::error::Error = &e;
    assert!(format!("{}", e).contains("unauthorized"));
}

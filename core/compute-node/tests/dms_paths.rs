use posemesh_compute_node::dms::DmsPaths;
use url::Url;
use uuid::Uuid;

#[test]
fn builds_tasks_and_task_specific_paths() {
    let base: Url = "https://dms.example".parse().unwrap();
    let paths = DmsPaths::new(base.clone());

    let tasks = paths.tasks();
    assert_eq!(tasks.as_str(), "https://dms.example/tasks");
    assert_eq!(tasks.path(), "/tasks");
    assert_eq!(tasks.domain(), base.domain());

    const CAP: &str = "/reconstruction/local-and-global-refinement/v1";
    let with_cap = paths.tasks_with_capability(CAP);
    assert_eq!(with_cap.path(), "/tasks");
    let qp: Vec<(String, String)> = with_cap
        .query_pairs()
        .map(|(k, v)| (k.into_owned(), v.into_owned()))
        .collect();
    assert!(qp.iter().any(|(k, v)| k == "capability" && v == CAP));

    let id = Uuid::new_v4();
    let hb = paths.heartbeat(id);
    assert_eq!(hb.path(), format!("/tasks/{}/heartbeat", id));
    let c = paths.complete(id);
    assert_eq!(c.path(), format!("/tasks/{}/complete", id));
    let f = paths.fail(id);
    assert_eq!(f.path(), format!("/tasks/{}/fail", id));
}

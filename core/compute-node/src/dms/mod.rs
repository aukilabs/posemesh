//! DMS path builders and DTO shells (no HTTP).

use url::Url;
use uuid::Uuid;

pub mod client;
pub mod types;

/// Helper to build DMS endpoint URLs.
#[derive(Clone, Debug)]
pub struct DmsPaths {
    base: Url,
}

impl DmsPaths {
    /// Create a new path builder from a base URL, e.g. `https://dms.example`.
    pub fn new(base: Url) -> Self {
        Self { base }
    }

    /// `GET/POST /tasks` — lease tasks (capability filtering via query).
    pub fn tasks(&self) -> Url {
        self.base.join("tasks").expect("join /tasks")
    }

    /// `GET/POST /tasks?capability=...` — lease by capability.
    pub fn tasks_with_capability(&self, capability: &str) -> Url {
        let mut url = self.tasks();
        {
            let mut qp = url.query_pairs_mut();
            qp.append_pair("capability", capability);
        }
        url
    }

    /// `POST /tasks/{id}/heartbeat`
    pub fn heartbeat(&self, task_id: Uuid) -> Url {
        self.base
            .join(&format!("tasks/{}/heartbeat", task_id))
            .expect("join heartbeat")
    }

    /// `POST /tasks/{id}/complete`
    pub fn complete(&self, task_id: Uuid) -> Url {
        self.base
            .join(&format!("tasks/{}/complete", task_id))
            .expect("join complete")
    }

    /// `POST /tasks/{id}/fail`
    pub fn fail(&self, task_id: Uuid) -> Url {
        self.base
            .join(&format!("tasks/{}/fail", task_id))
            .expect("join fail")
    }
}

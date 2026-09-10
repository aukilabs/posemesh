import base64
import contextlib
import io
import json
import time
from types import SimpleNamespace
import unittest
from unittest.mock import patch
import uuid

import submit


class SubmissionTests(unittest.TestCase):
    def test_dry_run_needs_no_token_and_cannot_contact_dms(self):
        argv = [
            "submit.py", "--dry-run",
            "--organization-id", str(uuid.uuid4()),
            "--domain-id", str(uuid.uuid4()),
            "--robot-peer-id", "robot", "--compute-peer-id", "compute",
        ]
        output = io.StringIO()
        with patch.dict(submit.os.environ, {}, clear=True), patch.object(submit.sys, "argv", argv), patch.object(submit, "Dms", side_effect=AssertionError("network access")), contextlib.redirect_stdout(output), contextlib.redirect_stderr(io.StringIO()):
            submit.main()
        self.assertEqual(json.loads(output.getvalue())["tasks"][0]["mode"], "dedicated")

    def test_completed_robot_cannot_publish_stale_readiness(self):
        class CompletedDms:
            def request(self, method, path):
                return {"tasks": [{"status": "completed", "meta": {"progress": {"phase": "ready"}}}]}

        with self.assertRaisesRegex(RuntimeError, "completed before"):
            submit.wait_ready(CompletedDms(), uuid.uuid4(), None, uuid.uuid4(), time.monotonic() + 1)

    def test_both_jobs_are_dedicated_without_a_completion_dependency(self):
        for capability in (submit.SERVE, submit.SEND):
            payload = submit.job(uuid.uuid4(), uuid.uuid4(), capability, {})
            self.assertEqual(payload["tasks"][0]["mode"], "dedicated")
            self.assertEqual(payload["tasks"][0]["max_attempts"], 1)
            self.assertEqual(payload["edges"], [])

    def test_both_jobs_include_required_dms_request_fields(self):
        # DMS CreateJobRequest requires priority, and CreateJobTaskRequest
        # requires its own label even when the enclosing job has a label.
        for capability in (submit.SERVE, submit.SEND):
            with self.subTest(capability=capability):
                payload = submit.job(uuid.uuid4(), uuid.uuid4(), capability, {})
                self.assertTrue({"label", "domain_id", "priority"} <= payload.keys())
                self.assertIsInstance(payload["priority"], int)
                self.assertGreaterEqual(payload["priority"], 0)
                task = payload["tasks"][0]
                self.assertTrue({"label", "stage", "capability", "max_attempts"} <= task.keys())
                self.assertIsInstance(task["label"], str)
                self.assertTrue(task["label"].strip())

    def test_ready_route_is_bound_to_both_peers_run_organization_and_domain(self):
        args = SimpleNamespace(organization_id=uuid.uuid4(), domain_id=uuid.uuid4(), robot_peer_id="robot", compute_peer_id="compute")
        run_id = uuid.uuid4()
        ready = {"run_id": str(run_id), "organization_id": str(args.organization_id), "domain_id": str(args.domain_id), "peer_id": "robot", "expected_compute_peer_id": "compute", "protocol": "/example/echo/1.0.0", "route": "/ip4/127.0.0.1/tcp/4001"}
        self.assertEqual(submit.validate_ready(ready, args, run_id), ready["route"])
        for field in ("run_id", "organization_id", "domain_id", "peer_id", "expected_compute_peer_id", "protocol", "route"):
            with self.subTest(field=field), self.assertRaises(ValueError):
                submit.validate_ready({**ready, field: "wrong"}, args, run_id)

    def test_job_token_must_match_the_configured_placement(self):
        organization_id, domain_id = uuid.uuid4(), uuid.uuid4()
        claims = {"org": str(organization_id), "domain_id": str(domain_id), "exp": 4102444800}
        token = "header." + base64.urlsafe_b64encode(json.dumps(claims).encode()).decode().rstrip("=") + ".signature"
        submit.token_scope(token, organization_id, domain_id)
        with self.assertRaises(ValueError):
            submit.token_scope(token, uuid.uuid4(), domain_id)
        with self.assertRaises(ValueError):
            submit.token_scope(token, organization_id, uuid.uuid4())

    def test_job_token_requires_the_dds_org_claim(self):
        organization_id, domain_id = uuid.uuid4(), uuid.uuid4()
        claims = {"organization_id": str(organization_id), "domain_id": str(domain_id), "exp": 4102444800}
        token = "header." + base64.urlsafe_b64encode(json.dumps(claims).encode()).decode().rstrip("=") + ".signature"
        with self.assertRaisesRegex(ValueError, "DDS-signed Domain job token"):
            submit.token_scope(token, organization_id, domain_id)


if __name__ == "__main__":
    unittest.main()

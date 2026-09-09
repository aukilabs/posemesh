#!/usr/bin/env python3
"""Submit two dedicated Echo tasks after the workers have been configured.

Use --dry-run to inspect the serve job without contacting any service.
Only public route hints are copied from the Robot's DMS progress to Compute.
"""

import argparse
import base64
import json
import os
from pathlib import Path
import sys
import time
import urllib.error
import urllib.parse
import urllib.request
import uuid


SERVE = "/examples/p2p-echo/serve/v1"
SEND = "/examples/p2p-echo/send/v1"


def job(domain_id, run_id, capability, meta):
    stage = "serve" if capability == SERVE else "send"
    return {
        "label": f"p2p-echo-{stage}-{run_id}",
        "domain_id": str(domain_id),
        "meta": {"example": "p2p-echo", "run_id": str(run_id)},
        "tasks": [{
            "stage": stage, "capability": capability, "capability_filters": {},
            "mode": "dedicated", "inputs_cids": [], "max_attempts": 1,
            "outputs_prefix": f"p2p-echo/{run_id}/{stage}/", "meta": meta,
        }],
        "edges": [],
    }


def token_scope(token, organization_id, domain_id):
    # This is a configuration check. DMS verifies the signature and authority.
    try:
        payload = token.split(".")[1]
        claims = json.loads(base64.urlsafe_b64decode(payload + "=" * (-len(payload) % 4)))
        if uuid.UUID(claims["org"]) != organization_id:
            raise ValueError("job token organization differs from ECHO_ORGANIZATION_ID")
        if uuid.UUID(claims["domain_id"]) != domain_id:
            raise ValueError("job token Domain differs from ECHO_DOMAIN_ID")
        if claims.get("exp", 0) <= time.time():
            raise ValueError("job token is expired or has no expiry")
    except (IndexError, KeyError, TypeError, json.JSONDecodeError) as error:
        raise ValueError("supply a DDS-signed Domain job token") from error


def validate_ready(progress, args, run_id):
    expected = {
        "run_id": str(run_id), "peer_id": args.robot_peer_id,
        "expected_compute_peer_id": args.compute_peer_id,
        "domain_id": str(args.domain_id), "organization_id": str(args.organization_id),
        "protocol": "/example/echo/1.0.0",
    }
    for key, value in expected.items():
        if progress.get(key) != value:
            raise ValueError(f"Robot readiness has a different {key}; check the two worker configurations")
    route = progress.get("route")
    if not isinstance(route, str) or not route.startswith("/"):
        raise ValueError("Robot did not report an advertised route")
    return route


class NoRedirect(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, req, fp, code, msg, headers, newurl):
        return None


class Dms:
    def __init__(self, base_url, token):
        parsed = urllib.parse.urlsplit(base_url)
        if parsed.scheme not in ("http", "https") or not parsed.hostname or parsed.username or parsed.password or parsed.query or parsed.fragment:
            raise ValueError("DMS_BASE_URL must be an HTTP(S) API base URL, including /v1")
        self.base = base_url.rstrip("/")
        self.token = token
        self.opener = urllib.request.build_opener(NoRedirect())

    def request(self, method, path, body=None):
        request = urllib.request.Request(
            self.base + path,
            data=None if body is None else json.dumps(body).encode(),
            headers={"Authorization": f"Bearer {self.token}", "Content-Type": "application/json"},
            method=method,
        )
        try:
            with self.opener.open(request, timeout=10) as response:
                raw = response.read()
                return json.loads(raw) if raw else {}
        except urllib.error.HTTPError as error:
            raise RuntimeError(f"DMS {method} {path} returned HTTP {error.code}") from None


def task_from_job(details):
    tasks = details.get("tasks", [])
    if len(tasks) != 1:
        raise ValueError("expected one Echo task in the job")
    task = tasks[0]
    if task.get("status") in ("failed", "canceled"):
        raise RuntimeError(f"Echo task {task['id']} is {task['status']}; inspect its DMS receipt")
    return task


def wait_ready(dms, job_id, args, run_id, deadline):
    while time.monotonic() < deadline:
        task = task_from_job(dms.request("GET", f"/jobs/{job_id}"))
        if task.get("status") == "completed":
            raise RuntimeError("Robot completed before the Compute task was submitted")
        progress = task.get("meta", {}).get("progress", {})
        if task.get("status") == "running" and progress.get("phase") == "ready":
            return validate_ready(progress, args, run_id)
        time.sleep(1)
    raise TimeoutError("Robot readiness timed out")


def wait_completed(dms, job_ids, deadline):
    while time.monotonic() < deadline:
        tasks = [task_from_job(dms.request("GET", f"/jobs/{job_id}")) for job_id in job_ids]
        if all(task.get("status") == "completed" for task in tasks):
            for task in tasks:
                print(json.dumps({"task_id": task["id"], "status": task["status"], "progress": task.get("meta", {}).get("progress")}))
            return
        time.sleep(1)
    raise TimeoutError("Echo tasks did not both complete before the deadline")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--organization-id", default=os.getenv("ECHO_ORGANIZATION_ID"), type=uuid.UUID, required=not os.getenv("ECHO_ORGANIZATION_ID"))
    parser.add_argument("--domain-id", default=os.getenv("ECHO_DOMAIN_ID"), type=uuid.UUID, required=not os.getenv("ECHO_DOMAIN_ID"))
    parser.add_argument("--compute-peer-id", default=os.getenv("ECHO_COMPUTE_PEER_ID"), required=not os.getenv("ECHO_COMPUTE_PEER_ID"))
    parser.add_argument("--robot-peer-id", default=os.getenv("ECHO_ROBOT_PEER_ID"), required=not os.getenv("ECHO_ROBOT_PEER_ID"))
    parser.add_argument("--message", default="hello over P2P")
    parser.add_argument("--timeout-seconds", type=int, default=120)
    parser.add_argument("--dry-run", action="store_true")
    args = parser.parse_args()
    if args.compute_peer_id == args.robot_peer_id:
        parser.error("Compute and Robot must have different Peer IDs")
    if not 1 <= args.timeout_seconds <= 600:
        parser.error("timeout-seconds must be 1..=600")
    run_id = uuid.uuid4()
    wire = json.dumps([str(run_id), args.message], ensure_ascii=False, separators=(",", ":")).encode()
    if not args.message or len(wire) > 1024:
        parser.error("message must be nonempty and fit the 1,024-byte Echo frame with its run ID")
    serve_job = job(args.domain_id, run_id, SERVE, {
        "run_id": str(run_id), "message": args.message,
        "expected_compute_peer_id": args.compute_peer_id, "timeout_seconds": args.timeout_seconds,
    })
    if args.dry_run:
        print(json.dumps(serve_job, indent=2))
        print("Dry run: no service contacted. The dedicated send job uses the Robot's validated readiness route.", file=sys.stderr)
        return

    token_path = os.getenv("APP_JWT_FILE")
    token = Path(token_path).read_text().strip() if token_path else os.environ["APP_JWT"].strip()
    token_scope(token, args.organization_id, args.domain_id)
    dms = Dms(os.environ["DMS_BASE_URL"], token)
    created = []
    deadline = time.monotonic() + args.timeout_seconds
    try:
        robot_job_id = str(uuid.UUID(dms.request("POST", "/jobs", serve_job)["job_id"]))
        created.append(robot_job_id)
        print(f"Robot job: {robot_job_id}", flush=True)
        route = wait_ready(dms, robot_job_id, args, run_id, deadline)
        send_job = job(args.domain_id, run_id, SEND, {
            "run_id": str(run_id), "message": args.message,
            "robot_peer_id": args.robot_peer_id, "route": route,
        })
        compute_job_id = str(uuid.UUID(dms.request("POST", "/jobs", send_job)["job_id"]))
        created.append(compute_job_id)
        print(f"Compute job: {compute_job_id}", flush=True)
        wait_completed(dms, created, deadline)
    except BaseException:
        for job_id in created:
            try:
                dms.request("POST", f"/jobs/{job_id}/cancel", {})
            except Exception:
                print(f"Could not cancel demo job {job_id}; inspect it in DMS.", file=sys.stderr)
        raise


if __name__ == "__main__":
    try:
        main()
    except (ValueError, RuntimeError, TimeoutError, KeyError, OSError) as error:
        sys.exit(str(error))

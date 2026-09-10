# Provision workers

Use this guide to create compute nodes and robots through DDS, assign a robot
to a Domain, or obtain a Domain access token for a job submitter. You need
`curl`, `jq`, and an organization operator account to provision workers.

With existing worker credentials and a robot assignment, continue with
[Configure workers](configure-workers.md). If you only need to submit jobs,
[authenticate with DDS](#authenticate-with-dds) and continue to
[Authorize job submission](#authorize-job-submission).

## Choose an environment

Set `API_BASE_URL`, `DDS_BASE_URL`, and `DMS_BASE_URL` to the same environment.
The DMS base includes `/v1`; the DDS base does not include `/api/v1`.
Set `DOMAIN_ID` to the Domain UUID. Use a Domain owned by the organization
that will own the workers.

## Check deployment support

This checkout needs providers with robot registration and Domain assignment,
dynamic dedicated capabilities, and task leases containing Domain storage
authority. P2P also needs DDS peer binding and DMS peer-bound lease issuance
and renewal for the chosen capability. A DMS build that only issues P2P
authority for a hard-coded reconstruction capability cannot run Echo.

For operators: robot support requires `DDS_ROBOT_WORKERS_ENABLED=true` in DDS
and `ROBOT_WORKERS_ENABLED=true` in DMS, with matching `DDS_ROBOT_AUDIENCE`.
Check the intended deployment's support; this repository does not enable those
services. Relay mode additionally needs DMS relay booking and an available relay.

## Authenticate with DDS

Use `curl` and `jq`. Prepare a private working directory outside the checkout:

~~~sh
umask 077
export WORKER_STATE_DIR="$HOME/.auki/worker-setup"
mkdir -p "$WORKER_STATE_DIR"
~~~

For email/password authentication, make these two requests. Both return JSON
with an `access_token` field:

| Request | Authentication or body | Save the response as |
| --- | --- | --- |
| `POST $API_BASE_URL/user/login` | JSON `{"email":"…","password":"…"}` | `$WORKER_STATE_DIR/login.json` |
| `POST $API_BASE_URL/service/domains-access-token` | Bearer `login.json`'s `access_token` | `$WORKER_STATE_DIR/dds-service.json` |

The second token is the DDS service bearer used to provision workers.
For example, after saving the login response:

~~~sh
jq -er '"Authorization: Bearer \(.access_token)"' \
  "$WORKER_STATE_DIR/login.json" > "$WORKER_STATE_DIR/login.headers"
curl --fail-with-body --silent --show-error --request POST \
  --header @"$WORKER_STATE_DIR/login.headers" \
  "$API_BASE_URL/service/domains-access-token" \
  --output "$WORKER_STATE_DIR/dds-service.json"
jq -er '"Authorization: Bearer \(.access_token)"' \
  "$WORKER_STATE_DIR/dds-service.json" > "$WORKER_STATE_DIR/dds.headers"
~~~

Keep these responses and headers private. The SDK's
[authentication guide](https://github.com/aukilabs/auki-sdk/blob/main/docs/how-to/authenticate.md)
covers other login types. Worker provisioning requires a User operator.
A job submitter can use a User or App service bearer with the right Domain
permissions; it can proceed to [Authorize job submission](#authorize-job-submission)
if it already has that bearer saved in `dds.headers`.

## Create a compute node

Create a dedicated node for Echo:

~~~sh
curl --fail-with-body --silent --show-error \
  --header @"$WORKER_STATE_DIR/dds.headers" \
  --header 'Content-Type: application/json' \
  --data '{"name":"echo-compute","mode":"dedicated"}' \
  "$DDS_BASE_URL/api/v1/nodes" --output "$WORKER_STATE_DIR/compute.json"
export REG_SECRET="$(jq -er '[.id, .registration_secret] | join(":") | @base64' \
  "$WORKER_STATE_DIR/compute.json")"
~~~

Despite its name, `REG_SECRET` takes **complete registration credentials**:
standard Base64 of `<node UUID>:<registration_secret>`. Passing only the raw
`registration_secret` response field fails authentication.

Save the complete registration credentials for the compute host. Choose your
own node name when provisioning another application. DDS sets the node's mode
and applies its wallet registration/staking policy; confirm your environment's
requirements. Continue with [compute configuration](configure-workers.md#compute-node).

## Create and assign a robot

Create the robot and save its complete, opaque credential:

~~~sh
curl --fail-with-body --silent --show-error \
  --header @"$WORKER_STATE_DIR/dds.headers" \
  --header 'Content-Type: application/json' \
  --data '{"name":"echo-robot","capabilities":["/examples/p2p-echo/serve/v1"]}' \
  "$DDS_BASE_URL/api/v1/robots" --output "$WORKER_STATE_DIR/robot.json"
export ROBOT_ID="$(jq -er '.robot.id' "$WORKER_STATE_DIR/robot.json")"
export ROBOT_REGISTRATION_CREDENTIALS_FILE="$WORKER_STATE_DIR/robot.credentials"
jq -er '.registration_credentials' "$WORKER_STATE_DIR/robot.json" \
  > "$ROBOT_REGISTRATION_CREDENTIALS_FILE"
jq -n --arg domain "$DOMAIN_ID" '{domain_id: $domain}' \
  > "$WORKER_STATE_DIR/assignment.json"
curl --fail-with-body --silent --show-error --request PUT \
  --header @"$WORKER_STATE_DIR/dds.headers" \
  --header 'Content-Type: application/json' \
  --data-binary @"$WORKER_STATE_DIR/assignment.json" \
  "$DDS_BASE_URL/api/v1/robots/$ROBOT_ID/assignment"
~~~

Save the returned credential unchanged for the robot host. For another runner,
replace the capability in the create request. Continue with
[robot configuration](configure-workers.md#robot).

Reassignment requires an offline robot with its leases and tokens expired.
Credential rotation is `POST /api/v1/robots/{id}/credentials/rotate`; securely
replace the local credential and restart the host. Files are read at startup.
The authoritative provisioning contracts live in
[DDS's node handler](https://github.com/aukilabs/domain-service/blob/main/dds/http/node.go)
and [robot handler](https://github.com/aukilabs/domain-service/blob/main/dds/http/robot.go).

## Authorize job submission

The job submitter uses a **Domain access token** to create and inspect DMS
jobs. The workers use their own registration credentials.

Use the private directory and DDS service bearer from
[Authenticate with DDS](#authenticate-with-dds), then exchange that bearer for
a token scoped to the task's Domain:

~~~sh
curl --fail-with-body --silent --show-error --request POST \
  --header @"$WORKER_STATE_DIR/dds.headers" \
  --header 'posemesh-client-id: posemesh-worker-tutorial' \
  "$DDS_BASE_URL/api/v1/domains/$DOMAIN_ID/auth" \
  --output "$WORKER_STATE_DIR/domain-auth.json"
export APP_JWT_FILE="$WORKER_STATE_DIR/domain-access-token"
jq -er '.access_token' "$WORKER_STATE_DIR/domain-auth.json" > "$APP_JWT_FILE"
~~~

DMS accepts a DDS-signed User or App Domain token with `domain:rw` or
`domain-data:rw`. Its `org` and `domain_id` must match the job. A read-only token,
worker credential, or P2P token cannot submit these jobs. Domain permissions
and Domain Server staking policy affect the scopes DDS issues.
Repeat this exchange when the token expires; renew the service bearer first
if needed. `APP_JWT_FILE` is the Echo helper's filename setting, even when the
bearer belongs to a User.

Use this file in the [Echo submission step](../tutorials/robot-and-compute.md#submit-the-work)
or when [submitting your own runner's task](write-a-runner.md#submit-a-task-and-read-its-artifact).

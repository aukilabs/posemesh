use crate::auth::token_manager::TokenProvider;
pub(crate) use crate::config::RelayBookingMode;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use reqwest::{
    header::{
        HeaderMap, HeaderName, HeaderValue, ACCEPT, AUTHORIZATION, CACHE_CONTROL, CONTENT_TYPE,
        LOCATION, RETRY_AFTER,
    },
    Client, Method, StatusCode,
};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::{collections::HashSet, fmt, sync::Arc, time::Duration};
use url::Url;
use uuid::Uuid;

const RELAY_HTTP_TIMEOUT: Duration = Duration::from_secs(10);
const MAX_RELAY_RESPONSE_BYTES: usize = 64 * 1024;
const MAX_IDEMPOTENCY_KEY_BYTES: usize = 128;
const MIN_BOOKING_DURATION_SECONDS: u64 = 300;
const MAX_BOOKING_DURATION_SECONDS: u64 = 86_400;
const MIN_RELAY_COUNT: u8 = 1;
const MAX_RELAY_COUNT: u8 = 3;
const MAX_RETRY_AFTER_SECONDS: u64 = 300;
const IDEMPOTENCY_KEY: HeaderName = HeaderName::from_static("idempotency-key");

/// Typed control-plane boundary owned by the Robot relay coordinator.
#[async_trait]
pub(crate) trait RelayBookingApi: Send + Sync {
    async fn active(&self) -> Result<Option<RelayBookingSnapshot>, RelayBookingClientError>;

    async fn create(
        &self,
        idempotency_key: &RelayIdempotencyKey,
        request: &CreateRelayBookingRequest,
    ) -> Result<CreateRelayBookingResponse, RelayBookingClientError>;

    async fn renew(
        &self,
        booking_id: Uuid,
    ) -> Result<RelayBookingSnapshot, RelayBookingClientError>;

    async fn report_reservation_failed(
        &self,
        booking_id: Uuid,
        request: &ReservationFailedRequest,
    ) -> Result<RelayBookingSnapshot, RelayBookingClientError>;

    async fn delete(&self, booking_id: Uuid) -> Result<(), RelayBookingClientError>;
}

/// Strict Robot client for the DMS relay-booking endpoints.
#[derive(Clone)]
pub(crate) struct RelayBookingClient {
    base: Url,
    http: Client,
    auth: Arc<dyn TokenProvider>,
}

impl RelayBookingClient {
    pub(crate) fn new(
        base: Url,
        auth: Arc<dyn TokenProvider>,
    ) -> Result<Self, RelayBookingClientError> {
        if base.cannot_be_a_base()
            || !matches!(base.scheme(), "http" | "https")
            || base.host_str().is_none()
            || !base.username().is_empty()
            || base.password().is_some()
            || base.query().is_some()
            || base.fragment().is_some()
        {
            return Err(RelayBookingClientError::InvalidConfiguration);
        }
        let http = Client::builder()
            .use_rustls_tls()
            .timeout(RELAY_HTTP_TIMEOUT)
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|_| RelayBookingClientError::InvalidConfiguration)?;
        Ok(Self { base, http, auth })
    }

    fn endpoint(
        &self,
        operation: RelayOperation,
        segments: &[&str],
    ) -> Result<Url, RelayBookingClientError> {
        let mut url = self.base.clone();
        url.path_segments_mut()
            .map_err(|_| RelayBookingClientError::InvalidConfiguration)?
            .pop_if_empty()
            .extend(segments.iter().copied());
        if url.query().is_some() || url.fragment().is_some() {
            return Err(RelayBookingClientError::InvalidResponse {
                operation,
                reason: "endpoint unexpectedly contains a query or fragment",
            });
        }
        Ok(url)
    }

    async fn send(
        &self,
        operation: RelayOperation,
        method: Method,
        url: Url,
        idempotency_key: Option<&RelayIdempotencyKey>,
        json_body: Option<Vec<u8>>,
    ) -> Result<RawResponse, RelayBookingClientError> {
        for attempt in 0..=1 {
            let bearer = self
                .auth
                .bearer()
                .await
                .map_err(|_| RelayBookingClientError::Authentication { operation })?;
            if bearer.is_empty() || bearer.bytes().any(|byte| byte.is_ascii_whitespace()) {
                return Err(RelayBookingClientError::Authentication { operation });
            }
            let mut authorization = HeaderValue::from_str(&format!("Bearer {bearer}"))
                .map_err(|_| RelayBookingClientError::Authentication { operation })?;
            authorization.set_sensitive(true);

            let mut request = self
                .http
                .request(method.clone(), url.clone())
                .header(AUTHORIZATION, authorization)
                .header(ACCEPT, HeaderValue::from_static("application/json"));
            if let Some(key) = idempotency_key {
                request = request.header(IDEMPOTENCY_KEY.clone(), key.header_value());
            }
            if let Some(body) = json_body.as_ref() {
                request = request
                    .header(CONTENT_TYPE, HeaderValue::from_static("application/json"))
                    .body(body.clone());
            }

            let response =
                request
                    .send()
                    .await
                    .map_err(|error| RelayBookingClientError::Transport {
                        operation,
                        timeout: error.is_timeout(),
                    })?;
            let raw = RawResponse::read(operation, response).await?;
            if raw.status == StatusCode::UNAUTHORIZED && attempt == 0 {
                self.auth.on_unauthorized().await;
                continue;
            }
            return Ok(raw);
        }
        unreachable!("the bounded authentication retry loop always returns")
    }
}

#[async_trait]
impl RelayBookingApi for RelayBookingClient {
    async fn active(&self) -> Result<Option<RelayBookingSnapshot>, RelayBookingClientError> {
        let operation = RelayOperation::Active;
        let url = self.endpoint(operation, &["relay-bookings", "active"])?;
        let expected_path = url.path().to_string();
        let raw = self.send(operation, Method::GET, url, None, None).await?;
        match raw.status {
            StatusCode::OK => {
                raw.require_no_control_headers(operation)?;
                let snapshot = raw.decode_snapshot(operation)?;
                if snapshot.state != RelayBookingState::Active {
                    return Err(RelayBookingClientError::InvalidResponse {
                        operation,
                        reason: "active endpoint returned a terminal booking",
                    });
                }
                Ok(Some(snapshot))
            }
            StatusCode::NO_CONTENT => {
                raw.require_empty(operation)?;
                raw.require_no_control_headers(operation)?;
                Ok(None)
            }
            _ => Err(raw.decode_http_error(operation, Some(&expected_path))?),
        }
    }

    async fn create(
        &self,
        idempotency_key: &RelayIdempotencyKey,
        request: &CreateRelayBookingRequest,
    ) -> Result<CreateRelayBookingResponse, RelayBookingClientError> {
        request.validate()?;
        let operation = RelayOperation::Create;
        let url = self.endpoint(operation, &["relay-bookings"])?;
        let collection_path = url.path().trim_end_matches('/').to_string();
        let body = serialize_request(operation, request)?;
        let raw = self
            .send(
                operation,
                Method::POST,
                url,
                Some(idempotency_key),
                Some(body),
            )
            .await?;
        match raw.status {
            StatusCode::CREATED | StatusCode::OK => {
                let disposition = if raw.status == StatusCode::CREATED {
                    RelayBookingCreateDisposition::Created
                } else {
                    RelayBookingCreateDisposition::Replayed
                };
                raw.require_no_retry_after(operation)?;
                let snapshot = raw.decode_snapshot(operation)?;
                if snapshot.mode != request.mode
                    || snapshot.requested_duration_seconds != request.requested_duration_seconds
                    || snapshot.relay_count != request.relay_count
                {
                    return Err(RelayBookingClientError::InvalidResponse {
                        operation,
                        reason: "create response does not match the requested booking policy",
                    });
                }
                let expected_location = format!("{collection_path}/{}", snapshot.booking_id);
                let location = raw.require_location(operation, &expected_location)?;
                Ok(CreateRelayBookingResponse {
                    disposition,
                    location,
                    snapshot,
                })
            }
            _ => Err(raw.decode_http_error(operation, Some(&collection_path))?),
        }
    }

    async fn renew(
        &self,
        booking_id: Uuid,
    ) -> Result<RelayBookingSnapshot, RelayBookingClientError> {
        let operation = RelayOperation::Renew;
        let booking_id_text = booking_id.to_string();
        let url = self.endpoint(operation, &["relay-bookings", &booking_id_text, "renew"])?;
        let expected_path = url.path().to_string();
        let raw = self.send(operation, Method::POST, url, None, None).await?;
        match raw.status {
            StatusCode::OK => {
                raw.require_no_control_headers(operation)?;
                let snapshot = raw.decode_snapshot(operation)?;
                require_booking_id(operation, booking_id, &snapshot)?;
                Ok(snapshot)
            }
            _ => Err(raw.decode_http_error(operation, Some(&expected_path))?),
        }
    }

    async fn report_reservation_failed(
        &self,
        booking_id: Uuid,
        request: &ReservationFailedRequest,
    ) -> Result<RelayBookingSnapshot, RelayBookingClientError> {
        request.validate()?;
        let operation = RelayOperation::ReservationFailed;
        let booking_id_text = booking_id.to_string();
        let url = self.endpoint(
            operation,
            &["relay-bookings", &booking_id_text, "reservation-failed"],
        )?;
        let expected_path = url.path().to_string();
        let body = serialize_request(operation, request)?;
        let raw = self
            .send(operation, Method::POST, url, None, Some(body))
            .await?;
        match raw.status {
            StatusCode::OK => {
                raw.require_no_control_headers(operation)?;
                let snapshot = raw.decode_snapshot(operation)?;
                require_booking_id(operation, booking_id, &snapshot)?;
                Ok(snapshot)
            }
            _ => Err(raw.decode_http_error(operation, Some(&expected_path))?),
        }
    }

    async fn delete(&self, booking_id: Uuid) -> Result<(), RelayBookingClientError> {
        let operation = RelayOperation::Delete;
        let booking_id_text = booking_id.to_string();
        let url = self.endpoint(operation, &["relay-bookings", &booking_id_text])?;
        let expected_path = url.path().to_string();
        let raw = self
            .send(operation, Method::DELETE, url, None, None)
            .await?;
        match raw.status {
            StatusCode::NO_CONTENT => {
                raw.require_empty(operation)?;
                raw.require_no_control_headers(operation)?;
                Ok(())
            }
            _ => Err(raw.decode_http_error(operation, Some(&expected_path))?),
        }
    }
}

fn require_booking_id(
    operation: RelayOperation,
    booking_id: Uuid,
    snapshot: &RelayBookingSnapshot,
) -> Result<(), RelayBookingClientError> {
    if snapshot.booking_id != booking_id {
        return Err(RelayBookingClientError::InvalidResponse {
            operation,
            reason: "response booking ID differs from the requested booking",
        });
    }
    Ok(())
}

fn serialize_request<T: Serialize>(
    operation: RelayOperation,
    request: &T,
) -> Result<Vec<u8>, RelayBookingClientError> {
    serde_json::to_vec(request)
        .map_err(|_| RelayBookingClientError::RequestSerialization { operation })
}

struct RawResponse {
    status: StatusCode,
    headers: HeaderMap,
    body: Vec<u8>,
}

impl RawResponse {
    async fn read(
        operation: RelayOperation,
        mut response: reqwest::Response,
    ) -> Result<Self, RelayBookingClientError> {
        require_no_store(operation, response.headers())?;
        if response
            .content_length()
            .is_some_and(|length| length > MAX_RELAY_RESPONSE_BYTES as u64)
        {
            return Err(RelayBookingClientError::ResponseTooLarge { operation });
        }
        let status = response.status();
        let headers = response.headers().clone();
        let mut body = Vec::new();
        while let Some(chunk) =
            response
                .chunk()
                .await
                .map_err(|error| RelayBookingClientError::Transport {
                    operation,
                    timeout: error.is_timeout(),
                })?
        {
            let new_length = body
                .len()
                .checked_add(chunk.len())
                .ok_or(RelayBookingClientError::ResponseTooLarge { operation })?;
            if new_length > MAX_RELAY_RESPONSE_BYTES {
                return Err(RelayBookingClientError::ResponseTooLarge { operation });
            }
            body.extend_from_slice(&chunk);
        }
        Ok(Self {
            status,
            headers,
            body,
        })
    }

    fn require_empty(&self, operation: RelayOperation) -> Result<(), RelayBookingClientError> {
        if !self.body.is_empty() {
            return Err(RelayBookingClientError::InvalidResponse {
                operation,
                reason: "no-content response carried a body",
            });
        }
        if self.headers.contains_key(CONTENT_TYPE) {
            return Err(RelayBookingClientError::InvalidResponse {
                operation,
                reason: "no-content response carried a content type",
            });
        }
        Ok(())
    }

    fn decode_snapshot(
        &self,
        operation: RelayOperation,
    ) -> Result<RelayBookingSnapshot, RelayBookingClientError> {
        self.require_json_content_type(operation)?;
        let snapshot: RelayBookingSnapshot = self.decode_json(operation)?;
        snapshot.validate(operation)?;
        Ok(snapshot)
    }

    fn decode_http_error(
        &self,
        operation: RelayOperation,
        expected_path: Option<&str>,
    ) -> Result<RelayBookingClientError, RelayBookingClientError> {
        if self.status.is_success() {
            return Err(RelayBookingClientError::InvalidResponse {
                operation,
                reason: "unexpected success status",
            });
        }
        self.require_json_content_type(operation)?;
        let body: RelayErrorResponse = self.decode_json(operation)?;
        if body.error.trim().is_empty() {
            return Err(RelayBookingClientError::InvalidResponse {
                operation,
                reason: "error response message is empty",
            });
        }
        validate_error_status(operation, self.status, body.code)?;

        let retry_after = parse_retry_after(operation, &self.headers)?;
        let raw_location = single_header(operation, &self.headers, LOCATION)?;
        let location = match body.code {
            RelayErrorCode::ActiveBookingConflict => {
                if operation != RelayOperation::Create {
                    return Err(RelayBookingClientError::InvalidResponse {
                        operation,
                        reason: "active-booking conflict returned by the wrong endpoint",
                    });
                }
                let collection = expected_path.ok_or(RelayBookingClientError::InvalidResponse {
                    operation,
                    reason: "missing expected collection path",
                })?;
                let expected = format!("{}/active", collection.trim_end_matches('/'));
                let actual = header_text(
                    operation,
                    raw_location.ok_or(RelayBookingClientError::InvalidResponse {
                        operation,
                        reason: "active-booking conflict omitted Location",
                    })?,
                )?;
                if actual != expected {
                    return Err(RelayBookingClientError::InvalidResponse {
                        operation,
                        reason: "active-booking conflict returned an invalid Location",
                    });
                }
                if retry_after.is_none() {
                    return Err(RelayBookingClientError::InvalidResponse {
                        operation,
                        reason: "active-booking conflict omitted Retry-After",
                    });
                }
                Some(expected)
            }
            RelayErrorCode::TargetPeerConflict => {
                if raw_location.is_some() || retry_after.is_none() {
                    return Err(RelayBookingClientError::InvalidResponse {
                        operation,
                        reason: "target-peer conflict control headers are invalid",
                    });
                }
                None
            }
            _ => {
                if raw_location.is_some() || retry_after.is_some() {
                    return Err(RelayBookingClientError::InvalidResponse {
                        operation,
                        reason: "unexpected Location or Retry-After header",
                    });
                }
                None
            }
        };

        Ok(RelayBookingClientError::Http {
            operation,
            status: self.status,
            code: body.code,
            retry_after,
            location,
        })
    }

    fn decode_json<T: DeserializeOwned>(
        &self,
        operation: RelayOperation,
    ) -> Result<T, RelayBookingClientError> {
        serde_json::from_slice(&self.body).map_err(|_| RelayBookingClientError::InvalidResponse {
            operation,
            reason: "response body is not the exact expected JSON document",
        })
    }

    fn require_json_content_type(
        &self,
        operation: RelayOperation,
    ) -> Result<(), RelayBookingClientError> {
        let value = single_header(operation, &self.headers, CONTENT_TYPE)?.ok_or(
            RelayBookingClientError::InvalidResponse {
                operation,
                reason: "JSON response omitted Content-Type",
            },
        )?;
        let value = header_text(operation, value)?;
        if !value.eq_ignore_ascii_case("application/json") {
            return Err(RelayBookingClientError::InvalidResponse {
                operation,
                reason: "response Content-Type is not application/json",
            });
        }
        Ok(())
    }

    fn require_location(
        &self,
        operation: RelayOperation,
        expected: &str,
    ) -> Result<String, RelayBookingClientError> {
        let value = single_header(operation, &self.headers, LOCATION)?.ok_or(
            RelayBookingClientError::InvalidResponse {
                operation,
                reason: "create response omitted Location",
            },
        )?;
        let value = header_text(operation, value)?;
        if value != expected {
            return Err(RelayBookingClientError::InvalidResponse {
                operation,
                reason: "create response Location does not identify its booking",
            });
        }
        Ok(expected.to_string())
    }

    fn require_no_retry_after(
        &self,
        operation: RelayOperation,
    ) -> Result<(), RelayBookingClientError> {
        if single_header(operation, &self.headers, RETRY_AFTER)?.is_some() {
            return Err(RelayBookingClientError::InvalidResponse {
                operation,
                reason: "success response carried Retry-After",
            });
        }
        Ok(())
    }

    fn require_no_control_headers(
        &self,
        operation: RelayOperation,
    ) -> Result<(), RelayBookingClientError> {
        if single_header(operation, &self.headers, LOCATION)?.is_some()
            || single_header(operation, &self.headers, RETRY_AFTER)?.is_some()
        {
            return Err(RelayBookingClientError::InvalidResponse {
                operation,
                reason: "response carried unexpected control headers",
            });
        }
        Ok(())
    }
}

fn require_no_store(
    operation: RelayOperation,
    headers: &HeaderMap,
) -> Result<(), RelayBookingClientError> {
    let value = single_header(operation, headers, CACHE_CONTROL)?.ok_or(
        RelayBookingClientError::InvalidResponse {
            operation,
            reason: "relay response omitted Cache-Control",
        },
    )?;
    if !header_text(operation, value)?.eq_ignore_ascii_case("no-store") {
        return Err(RelayBookingClientError::InvalidResponse {
            operation,
            reason: "relay response is not marked Cache-Control: no-store",
        });
    }
    Ok(())
}

fn single_header(
    operation: RelayOperation,
    headers: &HeaderMap,
    name: HeaderName,
) -> Result<Option<&HeaderValue>, RelayBookingClientError> {
    let mut values = headers.get_all(name).iter();
    let first = values.next();
    if values.next().is_some() {
        return Err(RelayBookingClientError::InvalidResponse {
            operation,
            reason: "response repeated a singleton control header",
        });
    }
    Ok(first)
}

fn header_text(
    operation: RelayOperation,
    value: &HeaderValue,
) -> Result<&str, RelayBookingClientError> {
    value
        .to_str()
        .map_err(|_| RelayBookingClientError::InvalidResponse {
            operation,
            reason: "response control header is not visible ASCII",
        })
}

fn parse_retry_after(
    operation: RelayOperation,
    headers: &HeaderMap,
) -> Result<Option<Duration>, RelayBookingClientError> {
    let Some(value) = single_header(operation, headers, RETRY_AFTER)? else {
        return Ok(None);
    };
    let seconds = header_text(operation, value)?
        .parse::<u64>()
        .ok()
        .filter(|seconds| (1..=MAX_RETRY_AFTER_SECONDS).contains(seconds))
        .ok_or(RelayBookingClientError::InvalidResponse {
            operation,
            reason: "Retry-After is not an integer between 1 and 300 seconds",
        })?;
    Ok(Some(Duration::from_secs(seconds)))
}

fn validate_error_status(
    operation: RelayOperation,
    status: StatusCode,
    code: RelayErrorCode,
) -> Result<(), RelayBookingClientError> {
    let valid_status = match code {
        RelayErrorCode::InvalidRequest
        | RelayErrorCode::MissingIdempotencyKey
        | RelayErrorCode::InvalidIdempotencyKey => status == StatusCode::BAD_REQUEST,
        RelayErrorCode::Unauthorized => status == StatusCode::UNAUTHORIZED,
        RelayErrorCode::InvalidRobotPrincipal
        | RelayErrorCode::InvalidRequesterPrincipal
        | RelayErrorCode::Forbidden => status == StatusCode::FORBIDDEN,
        RelayErrorCode::NotFound => status == StatusCode::NOT_FOUND,
        RelayErrorCode::ActiveBookingConflict
        | RelayErrorCode::IdempotencyConflict
        | RelayErrorCode::TargetPeerConflict
        | RelayErrorCode::StaleRobotPrincipal
        | RelayErrorCode::StaleRequesterPrincipal
        | RelayErrorCode::StaleFence => status == StatusCode::CONFLICT,
        RelayErrorCode::AuthorityEnded => status == StatusCode::GONE,
        RelayErrorCode::OrganizationSlotQuotaExceeded => status == StatusCode::TOO_MANY_REQUESTS,
        RelayErrorCode::DependencyUnavailable => status == StatusCode::SERVICE_UNAVAILABLE,
        RelayErrorCode::InternalError => status == StatusCode::INTERNAL_SERVER_ERROR,
    };
    if !valid_status {
        return Err(RelayBookingClientError::InvalidResponse {
            operation,
            reason: "relay error code does not match its HTTP status",
        });
    }

    let valid_operation = match operation {
        RelayOperation::Active => matches!(
            code,
            RelayErrorCode::Unauthorized
                | RelayErrorCode::InvalidRobotPrincipal
                | RelayErrorCode::InvalidRequesterPrincipal
                | RelayErrorCode::Forbidden
                | RelayErrorCode::StaleRobotPrincipal
                | RelayErrorCode::StaleRequesterPrincipal
                | RelayErrorCode::DependencyUnavailable
                | RelayErrorCode::InternalError
        ),
        RelayOperation::Create => matches!(
            code,
            RelayErrorCode::Unauthorized
                | RelayErrorCode::InvalidRequest
                | RelayErrorCode::MissingIdempotencyKey
                | RelayErrorCode::InvalidIdempotencyKey
                | RelayErrorCode::InvalidRobotPrincipal
                | RelayErrorCode::InvalidRequesterPrincipal
                | RelayErrorCode::Forbidden
                | RelayErrorCode::ActiveBookingConflict
                | RelayErrorCode::IdempotencyConflict
                | RelayErrorCode::TargetPeerConflict
                | RelayErrorCode::OrganizationSlotQuotaExceeded
                | RelayErrorCode::DependencyUnavailable
                | RelayErrorCode::InternalError
        ),
        RelayOperation::Renew => matches!(
            code,
            RelayErrorCode::Unauthorized
                | RelayErrorCode::InvalidRequest
                | RelayErrorCode::InvalidRobotPrincipal
                | RelayErrorCode::InvalidRequesterPrincipal
                | RelayErrorCode::Forbidden
                | RelayErrorCode::NotFound
                | RelayErrorCode::StaleRobotPrincipal
                | RelayErrorCode::StaleRequesterPrincipal
                | RelayErrorCode::AuthorityEnded
                | RelayErrorCode::DependencyUnavailable
                | RelayErrorCode::InternalError
        ),
        RelayOperation::ReservationFailed => matches!(
            code,
            RelayErrorCode::Unauthorized
                | RelayErrorCode::InvalidRequest
                | RelayErrorCode::InvalidRobotPrincipal
                | RelayErrorCode::InvalidRequesterPrincipal
                | RelayErrorCode::Forbidden
                | RelayErrorCode::NotFound
                | RelayErrorCode::StaleRobotPrincipal
                | RelayErrorCode::StaleRequesterPrincipal
                | RelayErrorCode::AuthorityEnded
                | RelayErrorCode::DependencyUnavailable
                | RelayErrorCode::InternalError
        ),
        RelayOperation::Delete => matches!(
            code,
            RelayErrorCode::Unauthorized
                | RelayErrorCode::InvalidRequest
                | RelayErrorCode::InvalidRobotPrincipal
                | RelayErrorCode::InvalidRequesterPrincipal
                | RelayErrorCode::Forbidden
                | RelayErrorCode::NotFound
                | RelayErrorCode::StaleRobotPrincipal
                | RelayErrorCode::StaleRequesterPrincipal
                | RelayErrorCode::AuthorityEnded
                | RelayErrorCode::DependencyUnavailable
                | RelayErrorCode::InternalError
        ),
    };
    if !valid_operation {
        return Err(RelayBookingClientError::InvalidResponse {
            operation,
            reason: "relay error code is not valid for this endpoint",
        });
    }
    Ok(())
}

/// Durable, locally validated DMS create idempotency key.
#[derive(Clone, PartialEq, Eq, Hash)]
pub(crate) struct RelayIdempotencyKey(String);

impl RelayIdempotencyKey {
    pub(crate) fn new(value: impl Into<String>) -> Result<Self, RelayBookingClientError> {
        let value = value.into();
        if value.is_empty()
            || value.len() > MAX_IDEMPOTENCY_KEY_BYTES
            || !value.bytes().all(|byte| byte.is_ascii_graphic())
        {
            return Err(RelayBookingClientError::InvalidIdempotencyKey);
        }
        Ok(Self(value))
    }

    fn header_value(&self) -> HeaderValue {
        HeaderValue::from_str(&self.0)
            .expect("a validated visible-ASCII idempotency key is always a valid header value")
    }
}

impl fmt::Debug for RelayIdempotencyKey {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("RelayIdempotencyKey([REDACTED])")
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CreateRelayBookingRequest {
    pub(crate) mode: RelayBookingMode,
    pub(crate) requested_duration_seconds: u64,
    pub(crate) relay_count: u8,
}

impl CreateRelayBookingRequest {
    pub(crate) fn new(
        mode: RelayBookingMode,
        requested_duration_seconds: u64,
        relay_count: u8,
    ) -> Result<Self, RelayBookingClientError> {
        let request = Self {
            mode,
            requested_duration_seconds,
            relay_count,
        };
        request.validate()?;
        Ok(request)
    }

    fn validate(&self) -> Result<(), RelayBookingClientError> {
        if !(MIN_BOOKING_DURATION_SECONDS..=MAX_BOOKING_DURATION_SECONDS)
            .contains(&self.requested_duration_seconds)
            || !(MIN_RELAY_COUNT..=MAX_RELAY_COUNT).contains(&self.relay_count)
        {
            return Err(RelayBookingClientError::InvalidRequest);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ReservationFailureReason {
    DialFailed,
    ReservationDenied,
    AddressMismatch,
    LimitMismatch,
    ReservationLost,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ReservationFailedRequest {
    pub(crate) slot_id: Uuid,
    pub(crate) assignment_id: Uuid,
    pub(crate) reservation_epoch: Uuid,
    pub(crate) reason: ReservationFailureReason,
}

impl ReservationFailedRequest {
    fn validate(&self) -> Result<(), RelayBookingClientError> {
        if self.slot_id.is_nil() || self.assignment_id.is_nil() || self.reservation_epoch.is_nil() {
            return Err(RelayBookingClientError::InvalidRequest);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RelayBookingCreateDisposition {
    Created,
    Replayed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CreateRelayBookingResponse {
    pub(crate) disposition: RelayBookingCreateDisposition,
    pub(crate) location: String,
    pub(crate) snapshot: RelayBookingSnapshot,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum RelayBookingState {
    Active,
    Canceled,
    Expired,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum RelaySlotState {
    Queued,
    Starting,
    Ready,
    Recovering,
    Reassigning,
    Ended,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RelayLimits {
    pub(crate) duration_seconds: u32,
    pub(crate) data_bytes_per_direction: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RelaySlotSnapshot {
    pub(crate) slot_id: Uuid,
    pub(crate) slot_index: u8,
    pub(crate) state: RelaySlotState,
    pub(crate) assignment_id: Option<Uuid>,
    pub(crate) reservation_epoch: Option<Uuid>,
    pub(crate) provider_peer_id: Option<String>,
    pub(crate) provider_base_addresses: Option<Vec<String>>,
    pub(crate) limits: Option<RelayLimits>,
    pub(crate) provider_lease_expires_at: Option<DateTime<Utc>>,
    pub(crate) recovery_expires_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RelayBookingSnapshot {
    pub(crate) booking_id: Uuid,
    pub(crate) mode: RelayBookingMode,
    pub(crate) state: RelayBookingState,
    pub(crate) relay_count: u8,
    pub(crate) requested_duration_seconds: u64,
    pub(crate) requested_until: DateTime<Utc>,
    pub(crate) authority_expires_at: DateTime<Utc>,
    pub(crate) assigned_count: u8,
    pub(crate) provider_ready_count: u8,
    pub(crate) unfilled_count: u8,
    pub(crate) created_at: DateTime<Utc>,
    pub(crate) ended_at: Option<DateTime<Utc>>,
    pub(crate) slots: Vec<RelaySlotSnapshot>,
}

impl RelayBookingSnapshot {
    fn validate(&self, operation: RelayOperation) -> Result<(), RelayBookingClientError> {
        let invalid = |reason| RelayBookingClientError::InvalidResponse { operation, reason };
        let observed_at = Utc::now();
        if self.booking_id.is_nil()
            || !(MIN_RELAY_COUNT..=MAX_RELAY_COUNT).contains(&self.relay_count)
            || !(MIN_BOOKING_DURATION_SECONDS..=MAX_BOOKING_DURATION_SECONDS)
                .contains(&self.requested_duration_seconds)
            || self.slots.len() != usize::from(self.relay_count)
        {
            return Err(invalid(
                "booking identity, policy, or slot count is invalid",
            ));
        }
        if self.requested_until <= self.created_at
            || self.authority_expires_at <= self.created_at
            || self.authority_expires_at > self.requested_until
        {
            return Err(invalid("booking deadlines are inconsistent"));
        }
        match self.state {
            RelayBookingState::Active if self.ended_at.is_some() => {
                return Err(invalid("active booking carries an end time"));
            }
            RelayBookingState::Active
                if self.requested_until <= observed_at
                    || self.authority_expires_at <= observed_at =>
            {
                return Err(invalid("active booking authority is not in the future"));
            }
            RelayBookingState::Canceled
            | RelayBookingState::Expired
            | RelayBookingState::Failed
                if self
                    .ended_at
                    .is_none_or(|ended_at| ended_at < self.created_at) =>
            {
                return Err(invalid("terminal booking has an invalid end time"));
            }
            _ => {}
        }

        let mut slot_ids = HashSet::new();
        let mut provider_peer_ids = HashSet::new();
        let mut assigned_count = 0usize;
        let mut provider_ready_count = 0usize;
        let mut unfilled_count = 0usize;
        for (expected_index, slot) in self.slots.iter().enumerate() {
            if slot.slot_id.is_nil()
                || slot.slot_index as usize != expected_index
                || !slot_ids.insert(slot.slot_id)
            {
                return Err(invalid("slot IDs and sorted indexes are invalid"));
            }
            if slot.assignment_id.is_some() != slot.reservation_epoch.is_some()
                || slot.assignment_id.is_some_and(|id| id.is_nil())
                || slot.reservation_epoch.is_some_and(|id| id.is_nil())
            {
                return Err(invalid("slot assignment fences are incomplete or invalid"));
            }
            assigned_count += usize::from(slot.assignment_id.is_some());
            provider_ready_count += usize::from(slot.state == RelaySlotState::Ready);
            unfilled_count += usize::from(matches!(
                slot.state,
                RelaySlotState::Queued | RelaySlotState::Reassigning
            ));

            if matches!(
                slot.state,
                RelaySlotState::Ready | RelaySlotState::Recovering
            ) {
                let peer_id = slot
                    .provider_peer_id
                    .as_deref()
                    .filter(|value| !value.trim().is_empty())
                    .ok_or_else(|| invalid("ready slot omitted its provider Peer ID"))?;
                if !provider_peer_ids.insert(peer_id) {
                    return Err(invalid("ready providers are not distinct"));
                }
                let bases = slot
                    .provider_base_addresses
                    .as_ref()
                    .filter(|values| !values.is_empty())
                    .ok_or_else(|| invalid("ready slot omitted provider base addresses"))?;
                let unique_bases = bases.iter().collect::<HashSet<_>>();
                if unique_bases.len() != bases.len()
                    || bases.iter().any(|base| base.trim().is_empty())
                {
                    return Err(invalid("ready slot provider bases are empty or duplicated"));
                }
                let limits = slot
                    .limits
                    .as_ref()
                    .ok_or_else(|| invalid("ready slot omitted finite limits"))?;
                if limits.duration_seconds == 0
                    || limits.data_bytes_per_direction == 0
                    || limits.data_bytes_per_direction > i64::MAX as u64
                {
                    return Err(invalid("ready slot limits are not finite and positive"));
                }
                if slot.assignment_id.is_none() {
                    return Err(invalid("ready slot omitted assignment fences"));
                }
                match slot.state {
                    RelaySlotState::Ready => {
                        let lease = slot
                            .provider_lease_expires_at
                            .ok_or_else(|| invalid("ready slot omitted its provider lease"))?;
                        if lease <= observed_at
                            || lease > self.authority_expires_at
                            || lease > self.requested_until
                        {
                            return Err(invalid("ready slot provider lease is not usable"));
                        }
                    }
                    RelaySlotState::Recovering => {
                        let recovery = slot.recovery_expires_at.ok_or_else(|| {
                            invalid("recovering slot omitted its recovery deadline")
                        })?;
                        if slot.provider_lease_expires_at.is_some()
                            || recovery <= observed_at
                            || recovery > self.authority_expires_at
                            || recovery > self.requested_until
                        {
                            return Err(invalid("recovering slot deadlines are invalid"));
                        }
                    }
                    _ => {}
                }
            } else if slot.provider_peer_id.is_some()
                || slot.provider_base_addresses.is_some()
                || slot.limits.is_some()
            {
                return Err(invalid("non-ready slot exposed provider routing metadata"));
            }
        }

        if assigned_count != usize::from(self.assigned_count)
            || provider_ready_count != usize::from(self.provider_ready_count)
            || unfilled_count != usize::from(self.unfilled_count)
            || self.assigned_count > self.relay_count
            || self.provider_ready_count > self.relay_count
            || self.unfilled_count > self.relay_count
        {
            return Err(invalid("booking summary counts do not match its slots"));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum RelayErrorCode {
    Unauthorized,
    InvalidRequest,
    MissingIdempotencyKey,
    InvalidIdempotencyKey,
    InvalidRobotPrincipal,
    InvalidRequesterPrincipal,
    Forbidden,
    NotFound,
    ActiveBookingConflict,
    IdempotencyConflict,
    TargetPeerConflict,
    StaleRobotPrincipal,
    StaleRequesterPrincipal,
    StaleFence,
    AuthorityEnded,
    OrganizationSlotQuotaExceeded,
    DependencyUnavailable,
    InternalError,
}

impl RelayErrorCode {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Unauthorized => "unauthorized",
            Self::InvalidRequest => "invalid_request",
            Self::MissingIdempotencyKey => "missing_idempotency_key",
            Self::InvalidIdempotencyKey => "invalid_idempotency_key",
            Self::InvalidRobotPrincipal => "invalid_robot_principal",
            Self::InvalidRequesterPrincipal => "invalid_requester_principal",
            Self::Forbidden => "forbidden",
            Self::NotFound => "not_found",
            Self::ActiveBookingConflict => "active_booking_conflict",
            Self::IdempotencyConflict => "idempotency_conflict",
            Self::TargetPeerConflict => "target_peer_conflict",
            Self::StaleRobotPrincipal => "stale_robot_principal",
            Self::StaleRequesterPrincipal => "stale_requester_principal",
            Self::StaleFence => "stale_fence",
            Self::AuthorityEnded => "authority_ended",
            Self::OrganizationSlotQuotaExceeded => "organization_slot_quota_exceeded",
            Self::DependencyUnavailable => "dependency_unavailable",
            Self::InternalError => "internal_error",
        }
    }

    pub(crate) fn is_invalid_requester_principal(self) -> bool {
        matches!(
            self,
            Self::InvalidRobotPrincipal | Self::InvalidRequesterPrincipal
        )
    }

    pub(crate) fn is_stale_requester_principal(self) -> bool {
        matches!(
            self,
            Self::StaleRobotPrincipal | Self::StaleRequesterPrincipal
        )
    }
}

impl fmt::Display for RelayErrorCode {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RelayErrorResponse {
    code: RelayErrorCode,
    error: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RelayOperation {
    Active,
    Create,
    Renew,
    ReservationFailed,
    Delete,
}

impl fmt::Display for RelayOperation {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Active => "active booking lookup",
            Self::Create => "booking create",
            Self::Renew => "booking renewal",
            Self::ReservationFailed => "reservation failure report",
            Self::Delete => "booking delete",
        })
    }
}

/// Body-redacted failures returned by [`RelayBookingApi`].
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub(crate) enum RelayBookingClientError {
    #[error("invalid relay booking client configuration")]
    InvalidConfiguration,
    #[error("invalid relay booking request")]
    InvalidRequest,
    #[error("Idempotency-Key must contain 1..=128 visible ASCII bytes")]
    InvalidIdempotencyKey,
    #[error("relay authentication is unavailable during {operation}")]
    Authentication { operation: RelayOperation },
    #[error("relay HTTP transport failed during {operation} (timeout: {timeout})")]
    Transport {
        operation: RelayOperation,
        timeout: bool,
    },
    #[error("failed to serialize the relay request during {operation}")]
    RequestSerialization { operation: RelayOperation },
    #[error("relay response exceeded the bounded body size during {operation}")]
    ResponseTooLarge { operation: RelayOperation },
    #[error("invalid relay response during {operation}: {reason}")]
    InvalidResponse {
        operation: RelayOperation,
        reason: &'static str,
    },
    #[error("DMS relay endpoint returned {status} ({code}) during {operation}")]
    Http {
        operation: RelayOperation,
        status: StatusCode,
        code: RelayErrorCode,
        retry_after: Option<Duration>,
        location: Option<String>,
    },
}

impl RelayBookingClientError {
    pub(crate) fn is_retryable(&self) -> bool {
        match self {
            Self::Transport { .. } => true,
            Self::Http { status, .. } => {
                *status == StatusCode::TOO_MANY_REQUESTS || status.is_server_error()
            }
            _ => false,
        }
    }

    pub(crate) fn http_code(&self) -> Option<RelayErrorCode> {
        match self {
            Self::Http { code, .. } => Some(*code),
            _ => None,
        }
    }

    pub(crate) fn retry_after(&self) -> Option<Duration> {
        match self {
            Self::Http { retry_after, .. } => *retry_after,
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::token_manager::{TokenProviderError, TokenProviderResult};
    use httpmock::{prelude::*, Then};
    use serde_json::{json, Value};
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[derive(Clone)]
    struct StaticProvider(&'static str);

    #[async_trait]
    impl TokenProvider for StaticProvider {
        async fn bearer(&self) -> TokenProviderResult<String> {
            Ok(self.0.to_string())
        }

        async fn on_unauthorized(&self) {}
    }

    #[derive(Clone)]
    struct RotatingProvider {
        tokens: Arc<Vec<&'static str>>,
        index: Arc<AtomicUsize>,
    }

    impl RotatingProvider {
        fn new(tokens: Vec<&'static str>) -> Self {
            Self {
                tokens: Arc::new(tokens),
                index: Arc::new(AtomicUsize::new(0)),
            }
        }
    }

    #[async_trait]
    impl TokenProvider for RotatingProvider {
        async fn bearer(&self) -> TokenProviderResult<String> {
            let index = self.index.load(Ordering::SeqCst);
            self.tokens
                .get(index)
                .map(|token| (*token).to_string())
                .ok_or_else(|| TokenProviderError::Message("no test token".to_string()))
        }

        async fn on_unauthorized(&self) {
            self.index.fetch_add(1, Ordering::SeqCst);
        }
    }

    fn make_client(server: &MockServer, auth: Arc<dyn TokenProvider>) -> RelayBookingClient {
        RelayBookingClient::new(server.base_url().parse().unwrap(), auth).unwrap()
    }

    fn booking_snapshot(booking_id: Uuid, state: RelayBookingState) -> Value {
        let created_at = Utc::now();
        let requested_until = created_at + chrono::Duration::hours(24);
        let authority_expires_at = created_at + chrono::Duration::minutes(5);
        let ended_at =
            (state != RelayBookingState::Active).then(|| created_at + chrono::Duration::minutes(1));
        json!({
            "booking_id": booking_id,
            "mode": "public",
            "state": state,
            "relay_count": 1,
            "requested_duration_seconds": 86_400,
            "requested_until": requested_until,
            "authority_expires_at": authority_expires_at,
            "assigned_count": 0,
            "provider_ready_count": 0,
            "unfilled_count": 1,
            "created_at": created_at,
            "ended_at": ended_at,
            "slots": [{
                "slot_id": Uuid::new_v4(),
                "slot_index": 0,
                "state": "queued"
            }]
        })
    }

    fn ready_booking_snapshot(booking_id: Uuid) -> (Value, Uuid, Uuid, Uuid) {
        let created_at = Utc::now();
        let slot_id = Uuid::new_v4();
        let assignment_id = Uuid::new_v4();
        let reservation_epoch = Uuid::new_v4();
        (
            json!({
                "booking_id": booking_id,
                "mode": "public",
                "state": "active",
                "relay_count": 1,
                "requested_duration_seconds": 86_400,
                "requested_until": created_at + chrono::Duration::hours(24),
                "authority_expires_at": created_at + chrono::Duration::minutes(5),
                "assigned_count": 1,
                "provider_ready_count": 1,
                "unfilled_count": 0,
                "created_at": created_at,
                "slots": [{
                    "slot_id": slot_id,
                    "slot_index": 0,
                    "state": "ready",
                    "assignment_id": assignment_id,
                    "reservation_epoch": reservation_epoch,
                    "provider_peer_id": "12D3KooWRelayClientTestPeer",
                    "provider_base_addresses": [
                        "/dns4/relay.example.com/tcp/443/p2p/12D3KooWRelayClientTestPeer"
                    ],
                    "limits": {
                        "duration_seconds": 900,
                        "data_bytes_per_direction": 10485760
                    },
                    "provider_lease_expires_at": created_at + chrono::Duration::minutes(3)
                }]
            }),
            slot_id,
            assignment_id,
            reservation_epoch,
        )
    }

    fn relay_json(then: Then, status: u16, body: Value) -> Then {
        then.status(status)
            .header("cache-control", "no-store")
            .header("content-type", "application/json")
            .json_body(body)
    }

    #[tokio::test]
    async fn sends_the_exact_robot_booking_lifecycle_contract() {
        let server = MockServer::start();
        let token = "peer-bound-robot-machine-token";
        let booking_id = Uuid::new_v4();
        let (snapshot, slot_id, assignment_id, reservation_epoch) =
            ready_booking_snapshot(booking_id);

        let active = server.mock(|when, then| {
            when.method(GET)
                .path("/relay-bookings/active")
                .header("authorization", format!("Bearer {token}"));
            relay_json(then, 200, snapshot.clone());
        });
        let create = server.mock(|when, then| {
            when.method(POST)
                .path("/relay-bookings")
                .header("authorization", format!("Bearer {token}"))
                .header("idempotency-key", "durable-create-key")
                .header("content-type", "application/json")
                .json_body(json!({
                    "mode": "public",
                    "requested_duration_seconds": 86_400,
                    "relay_count": 1
                }));
            relay_json(then, 201, snapshot.clone())
                .header("location", format!("/relay-bookings/{booking_id}"));
        });
        let renew = server.mock(|when, then| {
            when.method(POST)
                .path(format!("/relay-bookings/{booking_id}/renew"))
                .header("authorization", format!("Bearer {token}"));
            relay_json(then, 200, snapshot.clone());
        });
        let failure = server.mock(|when, then| {
            when.method(POST)
                .path(format!("/relay-bookings/{booking_id}/reservation-failed"))
                .header("authorization", format!("Bearer {token}"))
                .header("content-type", "application/json")
                .json_body(json!({
                    "slot_id": slot_id,
                    "assignment_id": assignment_id,
                    "reservation_epoch": reservation_epoch,
                    "reason": "reservation_lost"
                }));
            relay_json(then, 200, snapshot.clone());
        });
        let delete = server.mock(|when, then| {
            when.method(DELETE)
                .path(format!("/relay-bookings/{booking_id}"))
                .header("authorization", format!("Bearer {token}"));
            then.status(204).header("cache-control", "no-store");
        });

        let client = make_client(&server, Arc::new(StaticProvider(token)));
        assert_eq!(
            client.active().await.unwrap().unwrap().booking_id,
            booking_id
        );
        let request = CreateRelayBookingRequest::new(RelayBookingMode::Public, 86_400, 1).unwrap();
        let key = RelayIdempotencyKey::new("durable-create-key").unwrap();
        let created = client.create(&key, &request).await.unwrap();
        assert_eq!(created.disposition, RelayBookingCreateDisposition::Created);
        assert_eq!(created.snapshot.booking_id, booking_id);
        assert_eq!(created.location, format!("/relay-bookings/{booking_id}"));
        assert_eq!(
            client.renew(booking_id).await.unwrap().booking_id,
            booking_id
        );
        let failed = client
            .report_reservation_failed(
                booking_id,
                &ReservationFailedRequest {
                    slot_id,
                    assignment_id,
                    reservation_epoch,
                    reason: ReservationFailureReason::ReservationLost,
                },
            )
            .await
            .unwrap();
        assert_eq!(failed.booking_id, booking_id);
        client.delete(booking_id).await.unwrap();

        active.assert_hits(1);
        create.assert_hits(1);
        renew.assert_hits(1);
        failure.assert_hits(1);
        delete.assert_hits(1);
    }

    #[tokio::test]
    async fn refreshes_once_on_401_and_returns_redacted_conflict_metadata() {
        const SECRET_BODY: &str = "server-secret-that-must-not-escape";
        let server = MockServer::start();
        let booking_id = Uuid::new_v4();
        let unauthorized = server.mock(|when, then| {
            when.method(GET)
                .path("/relay-bookings/active")
                .header("authorization", "Bearer robot-token-a");
            relay_json(
                then,
                401,
                json!({"code": "unauthorized", "error": SECRET_BODY}),
            );
        });
        let success = server.mock(|when, then| {
            when.method(GET)
                .path("/relay-bookings/active")
                .header("authorization", "Bearer robot-token-b");
            relay_json(
                then,
                200,
                booking_snapshot(booking_id, RelayBookingState::Active),
            );
        });
        let provider = RotatingProvider::new(vec!["robot-token-a", "robot-token-b"]);
        let client = make_client(&server, Arc::new(provider.clone()));
        assert_eq!(
            client.active().await.unwrap().unwrap().booking_id,
            booking_id
        );
        assert_eq!(provider.index.load(Ordering::SeqCst), 1);
        unauthorized.assert_hits(1);
        success.assert_hits(1);

        let conflict_server = MockServer::start();
        let conflict = conflict_server.mock(|when, then| {
            when.method(POST)
                .path("/relay-bookings")
                .header("authorization", "Bearer robot-token-b")
                .header("idempotency-key", "fresh-key");
            relay_json(
                then,
                409,
                json!({"code": "active_booking_conflict", "error": SECRET_BODY}),
            )
            .header("location", "/relay-bookings/active")
            .header("retry-after", "17");
        });
        let conflict_client =
            make_client(&conflict_server, Arc::new(StaticProvider("robot-token-b")));
        let error = conflict_client
            .create(
                &RelayIdempotencyKey::new("fresh-key").unwrap(),
                &CreateRelayBookingRequest::new(RelayBookingMode::Public, 86_400, 1).unwrap(),
            )
            .await
            .unwrap_err();
        assert_eq!(
            error.http_code(),
            Some(RelayErrorCode::ActiveBookingConflict)
        );
        assert_eq!(error.retry_after(), Some(Duration::from_secs(17)));
        assert!(matches!(
            &error,
            RelayBookingClientError::Http {
                location: Some(location),
                ..
            } if location == "/relay-bookings/active"
        ));
        let rendered = format!("{error:?} {error}");
        for secret in [SECRET_BODY, "robot-token-a", "robot-token-b", "fresh-key"] {
            assert!(!rendered.contains(secret), "client error leaked {secret}");
        }
        conflict.assert_hits(1);
    }

    #[tokio::test]
    async fn rejects_cacheable_oversized_and_non_exact_responses_without_echoing_bodies() {
        let missing_no_store_server = MockServer::start();
        missing_no_store_server.mock(|when, then| {
            when.method(GET).path("/relay-bookings/active");
            then.status(204);
        });
        let error = make_client(
            &missing_no_store_server,
            Arc::new(StaticProvider("robot-token")),
        )
        .active()
        .await
        .unwrap_err();
        assert!(matches!(
            error,
            RelayBookingClientError::InvalidResponse { .. }
        ));

        let oversized_server = MockServer::start();
        oversized_server.mock(|when, then| {
            when.method(GET).path("/relay-bookings/active");
            then.status(500)
                .header("cache-control", "no-store")
                .header("content-type", "application/json")
                .body("x".repeat(MAX_RELAY_RESPONSE_BYTES + 1));
        });
        let error = make_client(&oversized_server, Arc::new(StaticProvider("robot-token")))
            .active()
            .await
            .unwrap_err();
        assert!(matches!(
            error,
            RelayBookingClientError::ResponseTooLarge { .. }
        ));

        const ATTACKER_VALUE: &str = "attacker-controlled-secret-value";
        let non_exact_server = MockServer::start();
        let booking_id = Uuid::new_v4();
        let mut body = booking_snapshot(booking_id, RelayBookingState::Active);
        body.as_object_mut()
            .unwrap()
            .insert("unexpected".to_string(), json!(ATTACKER_VALUE));
        non_exact_server.mock(|when, then| {
            when.method(GET).path("/relay-bookings/active");
            relay_json(then, 200, body.clone());
        });
        let error = make_client(&non_exact_server, Arc::new(StaticProvider("robot-token")))
            .active()
            .await
            .unwrap_err();
        let rendered = format!("{error:?} {error}");
        assert!(matches!(
            error,
            RelayBookingClientError::InvalidResponse { .. }
        ));
        assert!(!rendered.contains(ATTACKER_VALUE));
    }

    #[tokio::test]
    async fn preserves_the_configured_api_prefix_and_validates_prefixed_locations() {
        let server = MockServer::start();
        let booking_id = Uuid::new_v4();
        let snapshot = booking_snapshot(booking_id, RelayBookingState::Active);
        let active = server.mock(|when, then| {
            when.method(GET).path("/v1/relay-bookings/active");
            then.status(204).header("cache-control", "no-store");
        });
        let create = server.mock(|when, then| {
            when.method(POST).path("/v1/relay-bookings");
            relay_json(then, 201, snapshot.clone())
                .header("location", format!("/v1/relay-bookings/{booking_id}"));
        });
        let base = format!("{}/v1", server.base_url()).parse().unwrap();
        let client =
            RelayBookingClient::new(base, Arc::new(StaticProvider("robot-token"))).unwrap();

        assert!(client.active().await.unwrap().is_none());
        let response = client
            .create(
                &RelayIdempotencyKey::new("prefixed-create").unwrap(),
                &CreateRelayBookingRequest::new(RelayBookingMode::Public, 86_400, 1).unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(
            response.location,
            format!("/v1/relay-bookings/{booking_id}")
        );
        active.assert_hits(1);
        create.assert_hits(1);
    }

    #[tokio::test]
    async fn rejects_redirects_and_error_codes_from_the_wrong_endpoint() {
        let redirect_server = MockServer::start();
        let redirect = redirect_server.mock(|when, then| {
            when.method(GET).path("/relay-bookings/active");
            then.status(302)
                .header("cache-control", "no-store")
                .header("location", "/redirect-target");
        });
        let target = redirect_server.mock(|when, then| {
            when.method(GET).path("/redirect-target");
            then.status(204).header("cache-control", "no-store");
        });
        let error = make_client(&redirect_server, Arc::new(StaticProvider("robot-token")))
            .active()
            .await
            .unwrap_err();
        assert!(matches!(
            error,
            RelayBookingClientError::InvalidResponse { .. }
        ));
        redirect.assert_hits(1);
        target.assert_hits(0);

        let wrong_code_server = MockServer::start();
        wrong_code_server.mock(|when, then| {
            when.method(GET).path("/relay-bookings/active");
            relay_json(
                then,
                409,
                json!({"code": "target_peer_conflict", "error": "conflict"}),
            )
            .header("retry-after", "300");
        });
        let error = make_client(&wrong_code_server, Arc::new(StaticProvider("robot-token")))
            .active()
            .await
            .unwrap_err();
        assert!(matches!(
            error,
            RelayBookingClientError::InvalidResponse {
                reason: "relay error code is not valid for this endpoint",
                ..
            }
        ));
    }

    #[test]
    fn accepts_legacy_and_generic_requester_principal_codes_during_rollout() {
        for (encoded, code) in [
            (
                "invalid_robot_principal",
                RelayErrorCode::InvalidRobotPrincipal,
            ),
            (
                "invalid_requester_principal",
                RelayErrorCode::InvalidRequesterPrincipal,
            ),
        ] {
            let body: RelayErrorResponse = serde_json::from_value(json!({
                "code": encoded,
                "error": "principal rejected"
            }))
            .unwrap();
            assert_eq!(body.code, code);
            assert!(code.is_invalid_requester_principal());
            for operation in [
                RelayOperation::Active,
                RelayOperation::Create,
                RelayOperation::Renew,
                RelayOperation::ReservationFailed,
                RelayOperation::Delete,
            ] {
                validate_error_status(operation, StatusCode::FORBIDDEN, code).unwrap();
            }
        }

        for (encoded, code) in [
            ("stale_robot_principal", RelayErrorCode::StaleRobotPrincipal),
            (
                "stale_requester_principal",
                RelayErrorCode::StaleRequesterPrincipal,
            ),
        ] {
            let body: RelayErrorResponse = serde_json::from_value(json!({
                "code": encoded,
                "error": "principal is stale"
            }))
            .unwrap();
            assert_eq!(body.code, code);
            assert!(code.is_stale_requester_principal());
            for operation in [
                RelayOperation::Active,
                RelayOperation::Renew,
                RelayOperation::ReservationFailed,
                RelayOperation::Delete,
            ] {
                validate_error_status(operation, StatusCode::CONFLICT, code).unwrap();
            }
        }
    }

    #[test]
    fn locally_rejects_invalid_create_and_fence_inputs() {
        for key in [String::new(), "contains space".to_string(), "x".repeat(129)] {
            assert!(RelayIdempotencyKey::new(key).is_err());
        }
        for duration in [0, 299, 86_401] {
            assert!(CreateRelayBookingRequest::new(RelayBookingMode::Public, duration, 1).is_err());
        }
        for count in [0, 4] {
            assert!(
                CreateRelayBookingRequest::new(RelayBookingMode::Dedicated, 86_400, count).is_err()
            );
        }
        let invalid = ReservationFailedRequest {
            slot_id: Uuid::nil(),
            assignment_id: Uuid::new_v4(),
            reservation_epoch: Uuid::new_v4(),
            reason: ReservationFailureReason::DialFailed,
        };
        assert!(invalid.validate().is_err());
    }

    #[test]
    fn validates_ready_and_recovering_slot_deadline_shapes() {
        let booking_id = Uuid::new_v4();
        let (ready, _, _, _) = ready_booking_snapshot(booking_id);
        let ready: RelayBookingSnapshot = serde_json::from_value(ready).unwrap();
        ready.validate(RelayOperation::Active).unwrap();

        let mut recovering = serde_json::to_value(ready).unwrap();
        let root = recovering.as_object_mut().unwrap();
        root.insert("provider_ready_count".to_string(), json!(0));
        let slot = root
            .get_mut("slots")
            .and_then(Value::as_array_mut)
            .and_then(|slots| slots.first_mut())
            .and_then(Value::as_object_mut)
            .unwrap();
        slot.insert("state".to_string(), json!("recovering"));
        slot.remove("provider_lease_expires_at");
        slot.insert(
            "recovery_expires_at".to_string(),
            json!(Utc::now() + chrono::Duration::minutes(2)),
        );
        let recovering: RelayBookingSnapshot = serde_json::from_value(recovering).unwrap();
        recovering.validate(RelayOperation::Active).unwrap();

        let mut incomplete = recovering;
        incomplete.slots[0].limits = None;
        assert!(incomplete.validate(RelayOperation::Active).is_err());
    }
}

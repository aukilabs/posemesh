use std::{fmt, time::Duration};

use chrono::{DateTime, Utc};
use futures::{
    io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt},
    Future,
};
use libp2p::{PeerId, StreamProtocol};
use serde::{
    de::{self, MapAccess, Visitor},
    ser::SerializeStruct,
    Deserialize, Deserializer, Serialize, Serializer,
};
use uuid::Uuid;

use crate::{Error, Result};

pub(crate) const PROTOCOL: StreamProtocol = StreamProtocol::new("/auki-p2p/relay-auth/1");
pub(crate) const REQUEST_MAX_BYTES: usize = 64 * 1024;
pub(crate) const RESPONSE_MAX_BYTES: usize = 4 * 1024;
pub(crate) const TIMEOUT: Duration = Duration::from_secs(10);
pub(crate) const MAX_ACCEPTED_TTL: Duration = Duration::from_secs(30);

pub(crate) struct Request<'a> {
    pub(crate) domain_id: Uuid,
    pub(crate) target_peer_id: PeerId,
    pub(crate) p2p_access_token: &'a str,
}

impl fmt::Debug for Request<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Request")
            .field("version", &1)
            .field("domain_id", &self.domain_id)
            .field("target_peer_id", &self.target_peer_id)
            .field("p2p_access_token", &"[REDACTED]")
            .finish()
    }
}

impl Serialize for Request<'_> {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("RelayAdmissionRequest", 4)?;
        state.serialize_field("version", &1_u8)?;
        state.serialize_field("domain_id", &self.domain_id.to_string())?;
        state.serialize_field("target_peer_id", &self.target_peer_id.to_string())?;
        state.serialize_field("p2p_access_token", self.p2p_access_token)?;
        state.end()
    }
}

#[derive(Debug, PartialEq, Eq)]
struct Response {
    accepted: bool,
    accepted_until: Option<String>,
}

impl<'de> Deserialize<'de> for Response {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct ResponseVisitor;

        impl<'de> Visitor<'de> for ResponseVisitor {
            type Value = Response;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("an exact relay admission response object")
            }

            fn visit_map<A>(self, mut map: A) -> std::result::Result<Self::Value, A::Error>
            where
                A: MapAccess<'de>,
            {
                let mut accepted = None;
                let mut accepted_until = None;
                while let Some(key) = map.next_key::<String>()? {
                    match key.as_str() {
                        "accepted" => {
                            if accepted.is_some() {
                                return Err(de::Error::duplicate_field("accepted"));
                            }
                            accepted = Some(map.next_value::<bool>()?);
                        }
                        "accepted_until" => {
                            if accepted_until.is_some() {
                                return Err(de::Error::duplicate_field("accepted_until"));
                            }
                            accepted_until = Some(map.next_value::<String>()?);
                        }
                        _ => return Err(de::Error::unknown_field(&key, RESPONSE_FIELDS)),
                    }
                }
                let accepted = accepted.ok_or_else(|| de::Error::missing_field("accepted"))?;
                match (accepted, accepted_until) {
                    (true, Some(accepted_until)) => Ok(Response {
                        accepted,
                        accepted_until: Some(accepted_until),
                    }),
                    (false, None) => Ok(Response {
                        accepted,
                        accepted_until: None,
                    }),
                    _ => Err(de::Error::custom(
                        "accepted_until is required only for accepted responses",
                    )),
                }
            }
        }

        deserializer.deserialize_map(ResponseVisitor)
    }
}

const RESPONSE_FIELDS: &[&str] = &["accepted", "accepted_until"];

pub(crate) async fn authorize<S, F>(
    stream: &mut S,
    request: Request<'_>,
    now: F,
) -> Result<DateTime<Utc>>
where
    S: AsyncRead + AsyncWrite + Unpin,
    F: FnOnce() -> DateTime<Utc>,
{
    let payload = serde_json::to_vec(&request).map_err(|_| Error::RelayAdmissionMalformed)?;
    timeout(TIMEOUT, write_frame(stream, &payload, REQUEST_MAX_BYTES)).await?;
    let response = timeout(TIMEOUT, async {
        let response = read_frame(stream, RESPONSE_MAX_BYTES).await?;
        require_eof(stream).await?;
        Ok(response)
    })
    .await?;
    let response = decode_response(&response)?;
    let now = now();
    if !response.accepted {
        return Err(Error::RelayAdmissionDenied);
    }
    let raw_deadline = response
        .accepted_until
        .ok_or(Error::RelayAdmissionMalformed)?;
    let parsed =
        DateTime::parse_from_rfc3339(&raw_deadline).map_err(|_| Error::RelayAdmissionMalformed)?;
    if parsed.offset().local_minus_utc() != 0 {
        return Err(Error::RelayAdmissionMalformed);
    }
    let accepted_until = parsed.with_timezone(&Utc);
    if now >= accepted_until {
        return Err(Error::RelayAdmissionExpired);
    }
    let maximum = now
        + chrono::Duration::from_std(MAX_ACCEPTED_TTL)
            .map_err(|_| Error::RelayAdmissionMalformed)?;
    if accepted_until > maximum {
        return Err(Error::RelayAdmissionMalformed);
    }
    Ok(accepted_until)
}

async fn timeout<T>(duration: Duration, future: impl Future<Output = Result<T>>) -> Result<T> {
    tokio::time::timeout(duration, future)
        .await
        .map_err(|_| Error::RelayAdmissionTimeout)?
}

async fn write_frame<S>(stream: &mut S, payload: &[u8], maximum: usize) -> Result<()>
where
    S: AsyncWrite + Unpin,
{
    if payload.is_empty() || payload.len() > maximum {
        return Err(Error::RelayAdmissionFrameTooLarge { maximum });
    }
    let length =
        u32::try_from(payload.len()).map_err(|_| Error::RelayAdmissionFrameTooLarge { maximum })?;
    stream.write_all(&length.to_be_bytes()).await?;
    stream.write_all(payload).await?;
    stream.flush().await?;
    Ok(())
}

async fn read_frame<S>(stream: &mut S, maximum: usize) -> Result<Vec<u8>>
where
    S: AsyncRead + Unpin,
{
    let mut header = [0_u8; 4];
    stream.read_exact(&mut header).await?;
    let length = u32::from_be_bytes(header) as usize;
    if length == 0 || length > maximum {
        return Err(Error::RelayAdmissionFrameTooLarge { maximum });
    }
    let mut payload = vec![0; length];
    stream.read_exact(&mut payload).await?;
    Ok(payload)
}

async fn require_eof<S>(stream: &mut S) -> Result<()>
where
    S: AsyncRead + Unpin,
{
    let mut trailing = [0_u8; 1];
    match stream.read(&mut trailing).await? {
        0 => Ok(()),
        _ => Err(Error::RelayAdmissionMalformed),
    }
}

fn decode_response(payload: &[u8]) -> Result<Response> {
    std::str::from_utf8(payload).map_err(|_| Error::RelayAdmissionMalformed)?;
    let mut stream = serde_json::Deserializer::from_slice(payload).into_iter::<Response>();
    let response = stream
        .next()
        .ok_or(Error::RelayAdmissionMalformed)?
        .map_err(|_| Error::RelayAdmissionMalformed)?;
    if stream.byte_offset() != payload.len() {
        return Err(Error::RelayAdmissionMalformed);
    }
    Ok(response)
}

#[cfg(test)]
mod tests {
    use std::{
        io,
        pin::Pin,
        task::{Context, Poll},
    };

    use futures::io::{AsyncRead, AsyncWrite, Cursor};

    use super::*;

    const PEER_ID: &str = "12D3KooWBMyph6PCuP6GUJkwFdR7bLUPZ3exLvgEPpR93J52GaJg";
    const DOMAIN_ID: &str = "11111111-2222-3333-4444-555555555555";
    const TOKEN: &str = "header.payload.signature";
    const REQUEST_JSON: &str = "{\"version\":1,\"domain_id\":\"11111111-2222-3333-4444-555555555555\",\"target_peer_id\":\"12D3KooWBMyph6PCuP6GUJkwFdR7bLUPZ3exLvgEPpR93J52GaJg\",\"p2p_access_token\":\"header.payload.signature\"}";

    #[test]
    fn request_frame_vector_is_stable_and_redacted() {
        let request = request();
        let payload = serde_json::to_vec(&request).unwrap();
        assert_eq!(payload, REQUEST_JSON.as_bytes());
        assert_eq!((payload.len() as u32).to_be_bytes(), [0, 0, 0, 182]);
        let debug = format!("{request:?}");
        assert!(!debug.contains(TOKEN));
        assert!(debug.contains("[REDACTED]"));
    }

    #[test]
    fn response_decoder_is_exact() {
        let valid = br#"{"accepted":true,"accepted_until":"2030-01-02T03:04:05Z"}"#;
        assert_eq!(
            decode_response(valid).unwrap(),
            Response {
                accepted: true,
                accepted_until: Some("2030-01-02T03:04:05Z".into()),
            }
        );
        assert_eq!(
            decode_response(br#"{"accepted":false}"#).unwrap(),
            Response {
                accepted: false,
                accepted_until: None,
            }
        );

        for invalid in [
            br#"{}"#.as_slice(),
            br#"null"#,
            br#"{"accepted":null}"#,
            br#"{"Accepted":true,"accepted_until":"2030-01-02T03:04:05Z"}"#,
            br#"{"accepted":true,"accepted":false}"#,
            br#"{"accepted":true}"#,
            br#"{"accepted":false,"accepted_until":"2030-01-02T03:04:05Z"}"#,
            br#"{"accepted":true,"accepted_until":null}"#,
            br#"{"accepted":true,"accepted_until":"2030-01-02T03:04:05Z","extra":1}"#,
            br#"{"accepted":true,"accepted_until":"2030-01-02T03:04:05Z"} true"#,
            br#"{"accepted":true,"accepted_until":"2030-01-02T03:04:05Z"} "#,
            &[0xff],
        ] {
            assert!(
                decode_response(invalid).is_err(),
                "accepted invalid response: {invalid:?}"
            );
        }
    }

    #[tokio::test]
    async fn authorization_round_trip_enforces_denial_and_deadline() {
        let now = DateTime::parse_from_rfc3339("2030-01-02T03:04:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let mut accepted = ScriptedStream::with_response(frame(
            br#"{"accepted":true,"accepted_until":"2030-01-02T03:04:05Z"}"#,
        ));
        let deadline = authorize(&mut accepted, request(), || now).await.unwrap();
        assert_eq!(deadline.to_rfc3339(), "2030-01-02T03:04:05+00:00");
        assert_eq!(&accepted.written[4..], REQUEST_JSON.as_bytes());

        let mut denied = ScriptedStream::with_response(frame(br#"{"accepted":false}"#));
        assert!(matches!(
            authorize(&mut denied, request(), || now).await,
            Err(Error::RelayAdmissionDenied)
        ));

        let mut expired = ScriptedStream::with_response(frame(
            br#"{"accepted":true,"accepted_until":"2030-01-02T03:04:00Z"}"#,
        ));
        assert!(matches!(
            authorize(&mut expired, request(), || now).await,
            Err(Error::RelayAdmissionExpired)
        ));

        let mut overlong = ScriptedStream::with_response(frame(
            br#"{"accepted":true,"accepted_until":"2030-01-02T03:04:31Z"}"#,
        ));
        assert!(matches!(
            authorize(&mut overlong, request(), || now).await,
            Err(Error::RelayAdmissionMalformed)
        ));

        let completed_at = now + chrono::Duration::seconds(10);
        let mut slow_but_fresh = ScriptedStream::with_response(frame(
            br#"{"accepted":true,"accepted_until":"2030-01-02T03:04:31Z"}"#,
        ));
        assert_eq!(
            authorize(&mut slow_but_fresh, request(), || completed_at)
                .await
                .unwrap()
                .to_rfc3339(),
            "2030-01-02T03:04:31+00:00"
        );

        let mut extra_after_frame =
            frame(br#"{"accepted":true,"accepted_until":"2030-01-02T03:04:05Z"}"#);
        extra_after_frame.extend_from_slice(&[0]);
        let mut extra_after_frame = ScriptedStream::with_response(extra_after_frame);
        assert!(matches!(
            authorize(&mut extra_after_frame, request(), || now).await,
            Err(Error::RelayAdmissionMalformed)
        ));
    }

    #[tokio::test]
    async fn frame_bounds_are_enforced() {
        let mut sink = ScriptedStream::default();
        assert!(matches!(
            write_frame(&mut sink, &[], REQUEST_MAX_BYTES).await,
            Err(Error::RelayAdmissionFrameTooLarge { .. })
        ));
        let oversized = (RESPONSE_MAX_BYTES as u32 + 1).to_be_bytes().to_vec();
        let mut source = Cursor::new(oversized);
        assert!(matches!(
            read_frame(&mut source, RESPONSE_MAX_BYTES).await,
            Err(Error::RelayAdmissionFrameTooLarge { .. })
        ));
    }

    fn request() -> Request<'static> {
        Request {
            domain_id: Uuid::parse_str(DOMAIN_ID).unwrap(),
            target_peer_id: PEER_ID.parse().unwrap(),
            p2p_access_token: TOKEN,
        }
    }

    fn frame(payload: &[u8]) -> Vec<u8> {
        let mut framed = (payload.len() as u32).to_be_bytes().to_vec();
        framed.extend_from_slice(payload);
        framed
    }

    #[derive(Default)]
    struct ScriptedStream {
        response: Cursor<Vec<u8>>,
        written: Vec<u8>,
    }

    impl ScriptedStream {
        fn with_response(response: Vec<u8>) -> Self {
            Self {
                response: Cursor::new(response),
                written: Vec::new(),
            }
        }
    }

    impl AsyncRead for ScriptedStream {
        fn poll_read(
            mut self: Pin<&mut Self>,
            context: &mut Context<'_>,
            buffer: &mut [u8],
        ) -> Poll<io::Result<usize>> {
            Pin::new(&mut self.response).poll_read(context, buffer)
        }
    }

    impl AsyncWrite for ScriptedStream {
        fn poll_write(
            mut self: Pin<&mut Self>,
            _context: &mut Context<'_>,
            buffer: &[u8],
        ) -> Poll<io::Result<usize>> {
            self.written.extend_from_slice(buffer);
            Poll::Ready(Ok(buffer.len()))
        }

        fn poll_flush(self: Pin<&mut Self>, _context: &mut Context<'_>) -> Poll<io::Result<()>> {
            Poll::Ready(Ok(()))
        }

        fn poll_close(self: Pin<&mut Self>, _context: &mut Context<'_>) -> Poll<io::Result<()>> {
            Poll::Ready(Ok(()))
        }
    }
}

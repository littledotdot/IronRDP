//! Wire-compatible RDPEMT sub-header tests for live UDP auto-detect.
//!
//! Test is an explicit integration target because the ironrdp-client library
//! deliberately has `test = false`. It compiles the same production source
//! module, without exposing an additional public API.
#[path = "../src/udp_autodetect.rs"]
mod udp_autodetect;

use ironrdp_core::{MonotonicInstant, decode, encode_vec};
use ironrdp_pdu::rdp::autodetect::{AutoDetectRequest, AutoDetectResponse, BW_RESULTS_CONTINUOUS};
use ironrdp_rdpemt::{SubHeaderType, TunnelSubHeader};
use ironrdp_session::AutoDetectResponder;
use udp_autodetect::udp_autodetect_responses;

fn request_header(request: AutoDetectRequest) -> TunnelSubHeader {
    let bytes = encode_vec(&request).expect("encode auto-detect request");
    assert_eq!(bytes[1], SubHeaderType::AutoDetectRequest.to_u8());
    TunnelSubHeader { sub_header_type: SubHeaderType::AutoDetectRequest, data: bytes[2..].to_vec() }
}

fn response_pdu(header: &TunnelSubHeader) -> AutoDetectResponse {
    assert_eq!(header.sub_header_type, SubHeaderType::AutoDetectResponse);
    let mut bytes = vec![u8::try_from(header.wire_size()).expect("size"), header.sub_header_type.to_u8()];
    bytes.extend_from_slice(&header.data);
    decode::<AutoDetectResponse>(&bytes).expect("decode response")
}

#[test]
fn rtt_request_is_answered_in_udp_subheader_with_same_sequence_number() {
    let mut responder = AutoDetectResponder::default();
    let replies = udp_autodetect_responses(
        &[request_header(AutoDetectRequest::rtt_continuous(0x1234))],
        &mut responder, MonotonicInstant::from_millis(100),
    );
    assert_eq!(replies.len(), 1);
    assert_eq!(response_pdu(&replies[0]), AutoDetectResponse::RttResponse { sequence_number: 0x1234 });
}

#[test]
fn bandwidth_start_stop_preserves_measurement_and_returns_results_via_udp() {
    let mut responder = AutoDetectResponder::default();
    let start = request_header(AutoDetectRequest::bw_start_continuous(15));
    assert!(udp_autodetect_responses(
        &[start], &mut responder, MonotonicInstant::from_millis(100),
    ).is_empty());
    responder.record_bytes(1024);
    let stop = request_header(AutoDetectRequest::bw_stop_continuous(15));
    let replies = udp_autodetect_responses(
        &[stop], &mut responder, MonotonicInstant::from_millis(250),
    );
    assert_eq!(replies.len(), 1);
    assert_eq!(
        response_pdu(&replies[0]),
        AutoDetectResponse::BandwidthMeasureResults {
            sequence_number: 15,
            response_type: BW_RESULTS_CONTINUOUS,
            time_delta_ms: 150,
            byte_count: 1024,
        }
    );
}

#[test]
fn request_without_dvc_payload_still_needs_a_tunnel_response() {
    let message = ironrdp_rdpeudp_tokio::TunnelMessage {
        sub_headers: vec![request_header(AutoDetectRequest::rtt_continuous(8))],
        data: Vec::new(),
    };
    assert!(message.data.is_empty());
    let mut responder = AutoDetectResponder::default();
    let replies = udp_autodetect_responses(
        &message.sub_headers, &mut responder, MonotonicInstant::from_millis(100),
    );
    assert_eq!(replies.len(), 1, "empty higher-layer payload must not suppress auto-detect");
    assert_eq!(response_pdu(&replies[0]), AutoDetectResponse::RttResponse { sequence_number: 8 });
}

#[test]
fn malformed_subheader_is_ignored_without_poisoning_other_requests() {
    let invalid = TunnelSubHeader { sub_header_type: SubHeaderType::AutoDetectRequest, data: vec![0, 0] };
    let valid = request_header(AutoDetectRequest::rtt_continuous(4));
    let mut responder = AutoDetectResponder::default();
    let replies = udp_autodetect_responses(
        &[invalid, valid], &mut responder, MonotonicInstant::from_millis(100),
    );
    assert_eq!(replies.len(), 1);
    assert_eq!(response_pdu(&replies[0]), AutoDetectResponse::RttResponse { sequence_number: 4 });
}

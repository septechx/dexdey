use hmac::{Hmac, Mac};
use sha2::Sha256;
use uuid::Uuid;

use crate::protocol::varint::{decode_varint, encode_varint};

type HmacSha256 = Hmac<Sha256>;

const CHANNEL: &str = "velocity:player_info";

pub(crate) fn velocity_channel() -> &'static str {
    CHANNEL
}

pub(crate) struct ForwardingData {
    pub msg_id: i32,
    pub payload: Vec<u8>,
}

pub(crate) fn create_forwarding_data(
    secret: &[u8],
    address: &str,
    uuid: &Uuid,
    username: &str,
    properties: &[Property],
    _requested_version: i32,
) -> Vec<u8> {
    // TODO: Do something with requested_version
    let version = 1;

    let mut payload = Vec::new();
    encode_varint(&mut payload, version);
    write_string(&mut payload, address);
    payload.extend_from_slice(uuid.as_bytes());
    write_string(&mut payload, username);
    write_properties(&mut payload, properties);

    let sig = compute_hmac(secret, &payload);
    let mut result = Vec::with_capacity(32 + payload.len());
    result.extend_from_slice(&sig);
    result.extend_from_slice(&payload);
    result
}

fn compute_hmac(key: &[u8], data: &[u8]) -> [u8; 32] {
    let mut mac = HmacSha256::new_from_slice(key).expect("HMAC key should be valid");
    mac.update(data);
    mac.finalize().into_bytes().into()
}

pub(crate) fn try_parse_velocity_plugin_msg(
    frame: &[u8],
    compression: Option<i32>,
) -> Option<(i32, i32)> {
    let mut buf = frame;

    if compression.is_some() {
        let uncompressed_size = decode_varint(&mut buf).ok()?;
        if uncompressed_size != 0 {
            return None;
        }
    }

    let packet_id = decode_varint(&mut buf).ok()?;
    if packet_id != 0x04 {
        return None;
    }
    let msg_id = decode_varint(&mut buf).ok()?;
    let channel_len = decode_varint(&mut buf).ok()? as usize;
    if buf.len() < channel_len {
        return None;
    }
    let channel = std::str::from_utf8(&buf[..channel_len]).ok()?;
    if channel != CHANNEL {
        return None;
    }
    buf = &buf[channel_len..];
    let requested_version = if buf.len() == 1 { buf[0] as i32 } else { 1 };
    Some((msg_id, requested_version))
}

pub(crate) fn encode_login_plugin_response(
    msg_id: i32,
    data: &[u8],
    compression: Option<i32>,
) -> Vec<u8> {
    let mut buf = Vec::new();

    if compression.is_some() {
        encode_varint(&mut buf, 0);
    }

    encode_varint(&mut buf, 0x02);
    encode_varint(&mut buf, msg_id);
    buf.push(1);
    buf.extend_from_slice(data);
    buf
}

#[derive(Debug, Clone)]
pub(crate) struct Property {
    pub name: String,
    pub value: String,
    pub signature: Option<String>,
}

fn write_string(buf: &mut Vec<u8>, s: &str) {
    encode_varint(buf, s.len() as i32);
    buf.extend_from_slice(s.as_bytes());
}

fn write_properties(buf: &mut Vec<u8>, properties: &[Property]) {
    encode_varint(buf, properties.len() as i32);
    for prop in properties {
        write_string(buf, &prop.name);
        write_string(buf, &prop.value);
        if let Some(sig) = &prop.signature {
            buf.push(1);
            write_string(buf, sig);
        } else {
            buf.push(0);
        }
    }
}

pub(crate) fn offline_player_uuid(username: &str) -> Uuid {
    use md5::{Digest, Md5};
    let mut hasher = Md5::new();
    hasher.update(b"OfflinePlayer:");
    hasher.update(username.as_bytes());
    let hash = hasher.finalize();
    let mut bytes: [u8; 16] = hash.into();
    bytes[6] = (bytes[6] & 0x0f) | 0x30;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    Uuid::from_bytes(bytes)
}

pub(crate) async fn fetch_uuid(client: &reqwest::Client, username: &str) -> Result<Uuid, String> {
    let url = format!(
        "https://api.mojang.com/users/profiles/minecraft/{}",
        username
    );
    let resp = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("HTTP request failed: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("Mojang API returned {}", resp.status()));
    }
    let body: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("JSON parse failed: {e}"))?;
    let id = body["id"]
        .as_str()
        .ok_or_else(|| "missing 'id' field".to_string())?;
    let formatted = format!(
        "{}-{}-{}-{}-{}",
        &id[..8],
        &id[8..12],
        &id[12..16],
        &id[16..20],
        &id[20..]
    );
    Uuid::parse_str(&formatted).map_err(|e| format!("UUID parse failed: {e}"))
}

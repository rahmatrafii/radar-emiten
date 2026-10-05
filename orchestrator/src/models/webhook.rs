//! DTO toleran untuk payload webhook WhatsApp Cloud API.
//! Webhook bisa berupa pesan (`messages`) atau status delivery (`statuses`) — keduanya harus ditangani tanpa panic.
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct WebhookPayload {
    #[serde(default)]
    pub entry: Vec<Entry>,
}

#[derive(Debug, Deserialize)]
pub struct Entry {
    #[serde(default)]
    pub changes: Vec<Change>,
}

#[derive(Debug, Deserialize)]
pub struct Change {
    #[serde(default)]
    pub value: ChangeValue,
}

#[derive(Debug, Deserialize, Default)]
pub struct ChangeValue {
    #[serde(default)]
    pub messages: Vec<Message>,
    #[serde(default)]
    pub statuses: Vec<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct Message {
    pub id: String,
    pub from: String,
    #[serde(rename = "type", default)]
    pub kind: String,
    #[serde(default)]
    pub text: Option<TextBody>,
}

#[derive(Debug, Deserialize)]
pub struct TextBody {
    #[serde(default)]
    pub body: String,
}

pub enum Incoming {
    Text {
        wamid: String,
        from: String,
        body: String,
    },
    Unsupported {
        wamid: String,
        from: String,
        kind: String,
    },
}

/// Ekstrak pesan dari payload; abai terhadap entry/change tanpa messages.
pub fn extract_incoming(payload: &WebhookPayload) -> Vec<Incoming> {
    let mut out = Vec::new();
    for entry in &payload.entry {
        for change in &entry.changes {
            for msg in &change.value.messages {
                match (&msg.kind[..], &msg.text) {
                    ("text", Some(t)) if !t.body.trim().is_empty() => out.push(Incoming::Text {
                        wamid: msg.id.clone(),
                        from: msg.from.clone(),
                        body: t.body.clone(),
                    }),
                    _ => out.push(Incoming::Unsupported {
                        wamid: msg.id.clone(),
                        from: msg.from.clone(),
                        kind: if msg.kind.is_empty() {
                            "unknown".into()
                        } else {
                            msg.kind.clone()
                        },
                    }),
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_pesan_text() {
        let payload: WebhookPayload = serde_json::from_str(
            r#"{"entry":[{"changes":[{"value":{"messages":[{"id":"wamid.1","from":"62812","type":"text","text":{"body":"halo"}}]}}]}]}"#,
        )
        .unwrap();
        let items = extract_incoming(&payload);
        assert_eq!(items.len(), 1);
        match &items[0] {
            Incoming::Text { body, .. } => assert_eq!(body, "halo"),
            _ => panic!("harus text"),
        }
    }

    #[test]
    fn payload_status_saja_tidak_panic() {
        let payload: WebhookPayload = serde_json::from_str(
            r#"{"entry":[{"changes":[{"value":{"statuses":[{"id":"wamid.1","status":"delivered"}]}}]}]}"#,
        )
        .unwrap();
        assert!(extract_incoming(&payload).is_empty());
    }

    #[test]
    fn pesan_non_text_ditandai_unsupported() {
        let payload: WebhookPayload = serde_json::from_str(
            r#"{"entry":[{"changes":[{"value":{"messages":[{"id":"wamid.2","from":"62812","type":"image"}]}}]}]}"#,
        )
        .unwrap();
        match &extract_incoming(&payload)[0] {
            Incoming::Unsupported { kind, .. } => assert_eq!(kind, "image"),
            _ => panic!("harus unsupported"),
        }
    }
}

// Defensive patterns to replace common panic-prone code in lib.rs

use serde::{Deserialize, Serialize};
mod utils;

#[derive(Serialize, Deserialize, Debug, Default)]
struct Message {
    role: String,
    content: String,
}

// ── Safe input parsing ────────────────────────────────────────────────────────

// ❌ panics if input is empty or not a valid messages array
// let messages = utils::parse_messages(&input);

// ✅ graceful fallback
fn parse_input_safe(raw: &[u8]) -> Vec<Message> {
    if raw.is_empty() {
        utils::log("Warning: empty input");
        return vec![];
    }
    serde_json::from_slice(raw).unwrap_or_else(|e| {
        utils::log(&format!("Input parse error: {} — treating as plain text", e));
        vec![Message {
            role: "user".to_string(),
            content: String::from_utf8_lossy(raw).to_string(),
        }]
    })
}

// ── Safe LLM response parsing ─────────────────────────────────────────────────

fn extract_content(response_bytes: &[u8]) -> String {
    if response_bytes.is_empty() {
        utils::log("Warning: empty LLM response");
        return String::new();
    }
    let s = String::from_utf8_lossy(response_bytes);
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&s) {
        // OpenAI format
        if let Some(c) = json["choices"][0]["message"]["content"].as_str() {
            return c.to_string();
        }
        // UOMI format
        if let Some(c) = json["response"].as_str() {
            return c.to_string();
        }
    }
    s.to_string()
}

// ── Safe IPFS fetch ───────────────────────────────────────────────────────────

fn fetch_ipfs_safe(cid: &str) -> Option<String> {
    if cid.is_empty() {
        return None;
    }
    let bytes = utils::get_cid_file_service(cid.as_bytes().to_vec());
    if bytes.is_empty() {
        utils::log(&format!("Warning: empty response for CID {}", cid));
        return None;
    }
    Some(String::from_utf8_lossy(&bytes).to_string())
}

// ── Example run() using all safe helpers ─────────────────────────────────────

#[no_mangle]
pub extern "C" fn run() {
    let raw = utils::read_input();
    utils::log(&format!("Input size: {} bytes", raw.len()));

    let messages = parse_input_safe(&raw);
    if messages.is_empty() {
        // Always call save_output — even on error, so the host has something to return
        utils::save_output(b"{\"error\": \"No input received\"}");
        return;
    }

    let system = utils::system_message("You are a helpful UOMI agent.".to_string());
    let msgs_with_system = utils::process_messages(system, messages);

    let body = format!(
        "{{\"messages\": {}}}",
        serde_json::to_string(&msgs_with_system).unwrap()
    );
    utils::log("Calling AI service...");

    let response_bytes = utils::call_ai_service(1, utils::prepare_request(&body));
    utils::log(&format!("Response size: {} bytes", response_bytes.len()));

    // Always save output — pass through the raw response so the caller can parse it
    utils::save_output(&response_bytes);
}

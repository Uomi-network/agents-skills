// Pattern: RAG with IPFS
// Fetches a knowledge document from IPFS by CID, injects it as context,
// then answers the user's question grounded on that content.

use serde::{Deserialize, Serialize};

mod utils;

#[derive(Serialize, Deserialize, Debug)]
struct Message {
    role: String,
    content: String,
}

// Input: a messages array plus an optional CID pointing to the knowledge base
#[derive(Deserialize, Debug)]
struct RagInput {
    messages: Vec<Message>,
    knowledge_cid: Option<String>, // IPFS CID of the context document
}

#[no_mangle]
pub extern "C" fn run() {
    let raw = utils::read_input();

    let input: RagInput = serde_json::from_slice(&raw).unwrap_or_else(|_| {
        // Fallback: treat raw bytes as a plain messages array
        RagInput {
            messages: serde_json::from_slice(&raw).unwrap_or_default(),
            knowledge_cid: None,
        }
    });

    // Fetch the knowledge document from IPFS if a CID was provided
    let knowledge = match &input.knowledge_cid {
        Some(cid) => {
            utils::log(&format!("Fetching knowledge from IPFS CID: {}", cid));
            let bytes = utils::get_cid_file_service(cid.as_bytes().to_vec());
            let text = String::from_utf8_lossy(&bytes).to_string();
            utils::log(&format!("Fetched {} bytes from IPFS", text.len()));
            Some(text)
        }
        None => {
            // Try to read from the input file instead
            let file_bytes = utils::get_input_file_service();
            if !file_bytes.is_empty() {
                Some(String::from_utf8_lossy(&file_bytes).to_string())
            } else {
                None
            }
        }
    };

    // Build the system prompt, optionally injecting the knowledge document
    let system_content = match knowledge {
        Some(doc) => format!(
            "You are a knowledgeable assistant. \
             Answer questions strictly based on the following document.\n\n\
             --- DOCUMENT START ---\n{}\n--- DOCUMENT END ---\n\n\
             If the answer is not in the document, say so explicitly.",
            // Truncate to avoid hitting token limits
            &doc[..doc.len().min(12_000)]
        ),
        None => "You are a helpful assistant.".to_string(),
    };

    let system = utils::system_message(system_content);
    let messages_with_system = utils::process_messages(system, input.messages);

    let body = format!(
        "{{\"messages\": {}}}",
        serde_json::to_string(&messages_with_system).unwrap()
    );

    let response = utils::call_ai_service(1, utils::prepare_request(&body));
    utils::save_output(&response);
}

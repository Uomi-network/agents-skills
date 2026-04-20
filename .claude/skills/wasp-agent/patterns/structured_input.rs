// Pattern: Structured Input
// The agent receives a custom JSON object (not just messages) and processes it.
// Useful when the caller sends structured data like { "query": "...", "context": "..." }

use serde::{Deserialize, Serialize};

mod utils;

#[derive(Serialize, Deserialize, Debug)]
struct Message {
    role: String,
    content: String,
}

// Define your custom input shape
#[derive(Deserialize, Debug)]
struct AgentInput {
    query: String,
    context: Option<String>,
    language: Option<String>,
}

// Define your custom output shape
#[derive(Serialize)]
struct AgentOutput {
    answer: String,
    confidence: f32,
}

#[no_mangle]
pub extern "C" fn run() {
    let raw = utils::read_input();

    // Try to parse as structured input; fall back to raw string
    let input: AgentInput = match serde_json::from_slice(&raw) {
        Ok(v) => v,
        Err(e) => {
            utils::log(&format!("Failed to parse input: {}", e));
            // Treat the whole input as a plain query
            AgentInput {
                query: String::from_utf8_lossy(&raw).to_string(),
                context: None,
                language: None,
            }
        }
    };

    utils::log(&format!("Received query: {}", input.query));

    let lang = input.language.as_deref().unwrap_or("English");

    let system_content = format!(
        "You are a precise assistant. Always respond in {}. \
         Be concise and accurate.",
        lang
    );

    let user_content = match &input.context {
        Some(ctx) => format!("Context:\n{}\n\nQuestion: {}", ctx, input.query),
        None => input.query.clone(),
    };

    let messages = vec![
        Message { role: "system".to_string(), content: system_content },
        Message { role: "user".to_string(),   content: user_content   },
    ];

    let body = format!(
        "{{\"messages\": {}}}",
        serde_json::to_string(&messages).unwrap()
    );

    let response_bytes = utils::call_ai_service(1, utils::prepare_request(&body));
    let response_str = String::from_utf8_lossy(&response_bytes);

    // Parse the LLM response to extract just the text content
    let answer = extract_content(&response_str);

    // Save a structured JSON output
    let output = AgentOutput {
        answer,
        confidence: 1.0, // placeholder — could be derived from the model response
    };

    let output_json = serde_json::to_string(&output).unwrap();
    utils::save_output(output_json.as_bytes());
}

// Parse both UOMI and OpenAI response formats
fn extract_content(response: &str) -> String {
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(response) {
        // OpenAI format
        if let Some(content) = json["choices"][0]["message"]["content"].as_str() {
            return content.to_string();
        }
        // UOMI format
        if let Some(content) = json["response"].as_str() {
            return content.to_string();
        }
    }
    response.to_string()
}

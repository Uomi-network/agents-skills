// Pattern: Multi-step / Chained LLM Calls
// The agent calls the LLM more than once — first to classify or plan,
// then to execute or format. Useful for reasoning pipelines.

use serde::{Deserialize, Serialize};

mod utils;

#[derive(Serialize, Deserialize, Debug)]
struct Message {
    role: String,
    content: String,
}

#[derive(Serialize)]
struct FinalOutput {
    classification: String,
    answer: String,
}

#[no_mangle]
pub extern "C" fn run() {
    let raw = utils::read_input();
    let messages = utils::parse_messages(&raw);

    // Extract the last user message for classification
    let user_query = messages
        .iter()
        .rev()
        .find(|m| m.role == "user")
        .map(|m| m.content.clone())
        .unwrap_or_default();

    // ── Step 1: Classify the query ────────────────────────────────────────────
    utils::log("Step 1: classifying query...");

    let classify_messages = vec![
        Message {
            role: "system".to_string(),
            content: "Classify the user query into one of: FACTUAL, CREATIVE, MATH, OTHER. \
                      Reply with just the category name, nothing else."
                .to_string(),
        },
        Message {
            role: "user".to_string(),
            content: user_query.clone(),
        },
    ];

    let classify_body = format!(
        "{{\"messages\": {}}}",
        serde_json::to_string(&classify_messages).unwrap()
    );

    let classify_response = utils::call_ai_service(1, utils::prepare_request(&classify_body));
    let classification = extract_content(&String::from_utf8_lossy(&classify_response))
        .trim()
        .to_uppercase();

    utils::log(&format!("Classification: {}", classification));

    // ── Step 2: Answer with a prompt tuned for the category ──────────────────
    utils::log("Step 2: generating answer...");

    let system_content = match classification.as_str() {
        "FACTUAL" => "You are a precise, fact-based assistant. \
                      Cite sources or indicate if uncertain.",
        "CREATIVE" => "You are a creative assistant. \
                       Be imaginative, original, and expressive.",
        "MATH" => "You are a math assistant. \
                   Show step-by-step reasoning and verify your answer.",
        _ => "You are a helpful, versatile assistant.",
    };

    let system = utils::system_message(system_content.to_string());
    let answer_messages = utils::process_messages(system, messages);

    let answer_body = format!(
        "{{\"messages\": {}}}",
        serde_json::to_string(&answer_messages).unwrap()
    );

    let answer_response = utils::call_ai_service(1, utils::prepare_request(&answer_body));
    let answer = extract_content(&String::from_utf8_lossy(&answer_response));

    // ── Save combined output ──────────────────────────────────────────────────
    let output = FinalOutput {
        classification: classification.clone(),
        answer,
    };

    utils::save_output(serde_json::to_string(&output).unwrap().as_bytes());
}

fn extract_content(response: &str) -> String {
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(response) {
        if let Some(c) = json["choices"][0]["message"]["content"].as_str() {
            return c.to_string();
        }
        if let Some(c) = json["response"].as_str() {
            return c.to_string();
        }
    }
    response.to_string()
}

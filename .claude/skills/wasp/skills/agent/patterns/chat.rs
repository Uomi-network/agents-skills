// Pattern: Basic Chat Agent
// The simplest agent — receives messages, adds a system prompt, calls the LLM.

use serde::{Deserialize, Serialize};
use utils::log;

mod utils;

#[derive(Serialize, Deserialize, Debug)]
struct Message {
    role: String,
    content: String,
}

#[no_mangle]
pub extern "C" fn run() {
    let input = utils::read_input();
    let messages = utils::parse_messages(&input);

    let system = utils::system_message(
        "You are a helpful AI assistant on the UOMI network.".to_string(),
    );
    let messages_with_system = utils::process_messages(system, messages);

    let body = format!(
        "{{\"messages\": {}}}",
        serde_json::to_string(&messages_with_system).unwrap()
    );

    let response = utils::call_ai_service(1, utils::prepare_request(&body));
    utils::save_output(&response);
}

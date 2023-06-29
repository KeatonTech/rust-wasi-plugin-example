wit_bindgen::generate!({
    world: "plugin",
    path: "../plugin-host/simple-component.wit"
});

const SUGGESTION_STRINGS: [&str; 8] = [
    "Wasm",
    "WebAssembly",
    "WASI",
    "WasmTime",
    "Components",
    "Rust",
    "Software",
    "Plugin",
];

struct DemoAutocompleter {}

impl exports::simple_component::plugin::autocompleter::Autocompleter for DemoAutocompleter {
    fn generate_completions(
        input: wit_bindgen::rt::string::String,
    ) -> wit_bindgen::rt::vec::Vec<wit_bindgen::rt::string::String> {
        simple_component::plugin::logger::log_info(&format!(
            "Checking {} strings",
            SUGGESTION_STRINGS.len()
        ));
        let lowercase_input = input.to_ascii_lowercase();
        let result: Vec<String> = SUGGESTION_STRINGS
            .iter()
            .filter(|s| s.to_ascii_lowercase().contains(&lowercase_input))
            .map(|s| s.to_string())
            .collect();
        if result.is_empty() {
            simple_component::plugin::logger::log_error("No matches found!");
        }
        result
    }
}

export_plugin!(DemoAutocompleter);

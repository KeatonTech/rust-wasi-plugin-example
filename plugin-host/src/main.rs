use getch::Getch;
use std::io::stdout;
use std::io::Write;
use std::str::from_utf8;
use wasmtime::component::bindgen;
use wasmtime::component::Component;
use wasmtime::component::Linker;
use wasmtime::Store;
use wasmtime::{Config, Engine};
use wasmtime_wasi::preview2::Table;
use wasmtime_wasi::preview2::WasiCtx;
use wasmtime_wasi::preview2::WasiCtxBuilder;
use wasmtime_wasi::preview2::WasiView;

bindgen!({
    path: "./simple-component.wit",
    async: true,
});

const PLUGIN_FILE: &str = "../plugin-example/target/wasm32-wasi/debug/plugin_example.wasm";

#[derive(Copy, Clone)]
struct SimpleLogger {}

#[async_trait::async_trait]
impl simple_component::plugin::logger::Host for SimpleLogger {
    async fn log_info(&mut self, message: String) -> wasmtime::Result<()> {
        println!("[INFO] {}", message);
        Ok(())
    }

    async fn log_error(&mut self, message: String) -> wasmtime::Result<()> {
        println!("[ERR!] {}", message);
        Ok(())
    }
}

struct SimplePluginHostContext {
    logger: SimpleLogger,
    table: Table,
    context: WasiCtx,
}

impl WasiView for SimplePluginHostContext {
    fn table(&self) -> &wasmtime_wasi::preview2::Table {
        &self.table
    }

    fn table_mut(&mut self) -> &mut wasmtime_wasi::preview2::Table {
        &mut self.table
    }

    fn ctx(&self) -> &WasiCtx {
        &self.context
    }

    fn ctx_mut(&mut self) -> &mut WasiCtx {
        &mut self.context
    }
}

#[tokio::main]
async fn main() {
    // Create an 'engine', which is a struct that executes Wasm code
    let mut engine_config = Config::new();
    engine_config.wasm_component_model(true);
    engine_config.async_support(true);
    let engine = Engine::new(&engine_config).unwrap();

    // Create a 'linker', which associates WIT interfaces with concrete implementations
    let mut linker: Linker<SimplePluginHostContext> = Linker::new(&engine);
    wasmtime_wasi::preview2::wasi::command::add_to_linker(&mut linker).unwrap();
    Plugin::add_to_linker(&mut linker, |context| &mut context.logger).unwrap();

    // Load our plugin!
    let mut table = wasmtime_wasi::preview2::Table::new();
    let context = WasiCtxBuilder::new()
        .build(&mut table)
        .expect("Could not build WASI context");
    let mut store = Store::new(
        &engine,
        SimplePluginHostContext {
            logger: SimpleLogger {},
            table,
            context,
        },
    );
    let component = Component::from_file(&engine, PLUGIN_FILE).expect("Could not find plugin");
    let (plugin, _instance) = Plugin::instantiate_async(&mut store, &component, &linker)
        .await
        .expect("Could not instantiate plugin");
    let mut autocomplete_plugin = AutocompletePluginWrapper {
        plugin,
        store: &mut store,
    };

    let getch = Getch::new();
    let mut input = String::new();
    while let Ok(new_input) = input_looper(input, &getch).await {
        input = new_input;

        clear_console();
        if !input.is_empty() {
            generate_and_print_completions(&input, &mut autocomplete_plugin).await
        }
    }
}

struct AutocompletePluginWrapper<'a> {
    plugin: Plugin,
    store: &'a mut Store<SimplePluginHostContext>,
}

impl<'a> AutocompletePluginWrapper<'a> {
    async fn generate_completions(&mut self, for_string: &str) -> Vec<String> {
        self.plugin
            .simple_component_plugin_autocompleter()
            .call_generate_completions(&mut self.store, for_string)
            .await
            .unwrap()
    }
}

const KEYCODE_BACKSPACE: char = '\u{0008}';
const KEYCODE_DELETE: char = '\u{007F}';
const KEYCODE_ESCAPE: char = '\u{001B}';

async fn input_looper(input_so_far: String, getch: &Getch) -> Result<String, ()> {
    print!("Input: {}", input_so_far);
    stdout().lock().flush().unwrap();

    if let Ok(char_u8) = getch.getch() {
        println!();
        match char_u8 as char {
            KEYCODE_ESCAPE => Err(()),
            KEYCODE_BACKSPACE | KEYCODE_DELETE => {
                if input_so_far.len() > 1 {
                    Ok(input_so_far[0..=input_so_far.len() - 2].to_string())
                } else {
                    Ok("".to_string())
                }
            }
            'A'..='Z' | '1'..='9' | 'a'..='z' | ' ' => {
                Ok(input_so_far + from_utf8(&[char_u8]).unwrap())
            }
            _ => Ok(input_so_far),
        }
    } else {
        panic!("Encountered an error from getch");
    }
}

async fn generate_and_print_completions(
    input_so_far: &str,
    autocomplete_plugin: &mut AutocompletePluginWrapper<'_>,
) {
    let autocompletions = autocomplete_plugin.generate_completions(input_so_far).await;

    println!();
    println!("AUTOCOMPLETIONS =====================");
    for suggestion in autocompletions {
        println!("{}", suggestion);
    }
    println!("=====================================");
}

fn clear_console() {
    print!("{}[2J", 27 as char);
}

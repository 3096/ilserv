use clap::Parser;
use rouille::router;
use serde::Deserialize;

const MAIN_BASE: i64 = 0x7100000000;

struct SybmolEntry {
    address: i64,
    symbol: String,
}

#[derive(Deserialize, Debug)]
struct ScriptMethod {
    Address: i64,
    Name: String,
    Signature: String,
    TypeSignature: String,
}

#[derive(Deserialize, Debug)]
struct ScriptString {
    Address: i64,
    Value: String,
}

#[derive(Deserialize, Debug)]
struct ScriptMetadata {
    Address: i64,
    Name: String,
    Signature: Option<String>,
}

#[derive(Deserialize, Debug)]
struct ScriptMetadataMethod {
    Address: i64,
    Name: String,
    MethodAddress: i64,
}

#[derive(Deserialize, Debug)]
struct ScriptJson {
    ScriptMethod: Vec<ScriptMethod>,
    ScriptString: Vec<ScriptString>,
    ScriptMetadata: Vec<ScriptMetadata>,
    ScriptMetadataMethod: Vec<ScriptMetadataMethod>,
    Addresses: Vec<u64>,
}

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long, default_value = "script.json")]
    symbol_file: String,

    #[arg(short, long, default_value = "50204")]
    port: u16,
}

fn main() {
    let args = Args::parse();

    let json = std::fs::read_to_string(args.symbol_file).unwrap();
    let script_json_result: Result<ScriptJson, serde_json::Error> = serde_json::from_str(&json);
    let script_json = match script_json_result {
        Ok(script_json) => script_json,
        Err(e) => {
            eprintln!("Failed to parse JSON: {}", e);
            std::process::exit(1);
        }
    };

    let mut symbols = Vec::new();
    for method in script_json.ScriptMethod {
        symbols.push(SybmolEntry {
            address: method.Address,
            symbol: method.Signature,
        });
    }

    for string in script_json.ScriptString {
        symbols.push(SybmolEntry {
            address: string.Address,
            symbol: string.Value,
        });
    }

    for metadata in script_json.ScriptMetadata {
        symbols.push(SybmolEntry {
            address: metadata.Address,
            symbol: format!(
                "({}) {}",
                metadata.Signature.unwrap_or_default(),
                metadata.Name
            ),
        });
    }

    for metadata_method in script_json.ScriptMetadataMethod {
        symbols.push(SybmolEntry {
            address: metadata_method.Address,
            symbol: format!(
                "{} @ {:x}",
                metadata_method.Name,
                MAIN_BASE + metadata_method.MethodAddress
            ),
        });
    }

    symbols.sort_by(|a, b| a.address.cmp(&b.address));

    rouille::start_server(format!("localhost:{}", args.port), move |request| {
        router!(request,
            (GET) (/{address: String}) => {
                let address = match i64::from_str_radix(&address, 16) {
                    Ok(address) => address,
                    Err(_) => return rouille::Response::empty_404()
                };
                let relative_address = address - MAIN_BASE;
                let binary_search_result = symbols.binary_search_by(|symbol| symbol.address.cmp(&relative_address));
                if binary_search_result.is_ok() {
                    let symbol = &symbols[binary_search_result.unwrap()];
                    rouille::Response::text(format!("{:x} {}", MAIN_BASE + symbol.address, symbol.symbol))
                } else {
                    let found_index = binary_search_result.unwrap_err();
                    if found_index == 0 {
                        return rouille::Response::empty_404();
                    }
                    let symbol = &symbols[found_index - 1];
                    rouille::Response::text(format!("{:x}+{:x} {}", MAIN_BASE + symbol.address, relative_address - symbol.address, symbol.symbol))
                }
            },
            _ => rouille::Response::empty_404()
        )
    });
}

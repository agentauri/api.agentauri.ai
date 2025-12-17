//! OpenAPI Schema Export Binary
//!
//! This binary exports the OpenAPI specification as JSON to stdout.
//! Used for generating documentation with Docusaurus.
//!
//! Usage:
//!   cargo run -p api-gateway --bin export-openapi > openapi.json

use api_gateway::openapi::ApiDoc;
use utoipa::OpenApi;

fn main() {
    let openapi_json = ApiDoc::openapi()
        .to_pretty_json()
        .expect("Failed to serialize OpenAPI spec to JSON");

    println!("{}", openapi_json);
}

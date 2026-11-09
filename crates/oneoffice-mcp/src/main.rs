//! Unified ONEOFFICE MCP server binary entry point.

#![forbid(unsafe_code)]

use std::io::{self, BufReader};

use oneoffice_mcp::McpServer;

fn main() {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let reader = BufReader::new(stdin.lock());
    let writer = stdout.lock();
    let server = McpServer::new();

    if let Err(e) = server.run_stdio(reader, writer) {
        eprintln!("oneoffice-mcp server error: {e}");
        std::process::exit(1);
    }
}

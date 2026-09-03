pub mod server;

use std::path::Path;

pub fn run(path: &Path, port: u16, no_open: bool) -> std::io::Result<()> {
    server::serve(path, port, no_open)
}

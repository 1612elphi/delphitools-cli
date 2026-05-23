use crate::error::Error;
use std::io::{self, IsTerminal, Read};
use std::path::Path;

/// Read text input from: argument string, file path, or stdin.
/// Returns a usage error if stdin is a TTY and no other input is provided.
pub fn read_text(arg: Option<&str>) -> Result<String, Error> {
    if let Some(text) = arg {
        let path = Path::new(text);
        if path.is_file() {
            return std::fs::read_to_string(path)
                .map_err(|e| Error::Input(format!("{}: {e}", path.display())));
        }
        return Ok(text.to_string());
    }

    if !io::stdin().is_terminal() {
        let mut buf = String::new();
        io::stdin()
            .read_to_string(&mut buf)
            .map_err(|e| Error::Input(format!("stdin: {e}")))?;
        return Ok(buf);
    }

    Err(Error::Usage("no input provided".into()))
}

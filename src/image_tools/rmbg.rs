use crate::error::Error;
use std::path::{Path, PathBuf};

/// Background removal requires an ONNX segmentation model (e.g. BRIA RMBG-1.4).
/// We don't bundle one yet, so this tool returns a clear error instead of
/// silently doing nothing.
pub fn run(
    _images: &[PathBuf],
    _json: bool,
    _quiet: bool,
    _output: Option<&Path>,
) -> Result<(), Error> {
    Err(Error::Processing(
        "rmbg: not yet implemented.\n\
         Removing backgrounds requires an ONNX segmentation model that isn't bundled\n\
         with delphitools-cli yet. Track support at:\n\
         https://github.com/1612elphi/delphitools-cli"
            .into(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn returns_processing_error() {
        let err = run(&[], false, false, None).unwrap_err();
        assert!(matches!(err, Error::Processing(_)));
        let msg = format!("{err}");
        assert!(msg.contains("ONNX"));
        assert!(msg.contains("delphitools-cli"));
    }

    #[test]
    fn exits_with_code_three() {
        let err = run(&[PathBuf::from("anything.png")], false, false, None).unwrap_err();
        assert_eq!(err.exit_code(), 3);
    }
}

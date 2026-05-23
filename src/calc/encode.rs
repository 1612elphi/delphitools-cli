use crate::error::Error;
use crate::input;
use base64::{engine::general_purpose::STANDARD, Engine};
use md5::{Digest as Md5Digest, Md5};
use sha2::{Sha256, Sha512};

pub fn run_encode(encoding: &str, arg: Option<&str>) -> Result<(), Error> {
    let text = input::read_text(arg)?;
    match encoding {
        "base64" | "b64" => {
            println!("{}", STANDARD.encode(text.as_bytes()));
        }
        "url" => {
            println!("{}", urlencoding::encode(&text));
        }
        _ => {
            return Err(Error::Usage(format!(
                "unknown encoding: {encoding} (expected base64, url)"
            )));
        }
    }
    Ok(())
}

pub fn run_decode(encoding: &str, arg: Option<&str>) -> Result<(), Error> {
    let text = input::read_text(arg)?;
    let text = text.trim();
    match encoding {
        "base64" | "b64" => {
            let bytes = STANDARD
                .decode(text)
                .map_err(|e| Error::Input(format!("invalid base64: {e}")))?;
            let decoded = String::from_utf8(bytes)
                .map_err(|e| Error::Input(format!("decoded bytes are not valid UTF-8: {e}")))?;
            println!("{decoded}");
        }
        "url" => {
            let decoded = urlencoding::decode(text)
                .map_err(|e| Error::Input(format!("invalid URL encoding: {e}")))?;
            println!("{decoded}");
        }
        _ => {
            return Err(Error::Usage(format!(
                "unknown encoding: {encoding} (expected base64, url)"
            )));
        }
    }
    Ok(())
}

pub fn run_hash(algorithm: &str, arg: Option<&str>) -> Result<(), Error> {
    let text = input::read_text(arg)?;
    let hex = match algorithm {
        "md5" => {
            let mut hasher = Md5::new();
            hasher.update(text.as_bytes());
            format!("{:x}", hasher.finalize())
        }
        "sha256" => {
            let mut hasher = Sha256::new();
            hasher.update(text.as_bytes());
            format!("{:x}", hasher.finalize())
        }
        "sha512" => {
            let mut hasher = Sha512::new();
            hasher.update(text.as_bytes());
            format!("{:x}", hasher.finalize())
        }
        _ => {
            return Err(Error::Usage(format!(
                "unknown algorithm: {algorithm} (expected md5, sha256, sha512)"
            )));
        }
    };
    println!("{hex}");
    Ok(())
}

#[cfg(test)]
mod tests {
    use base64::{engine::general_purpose::STANDARD, Engine};
    use md5::{Digest as Md5Digest, Md5};
    use sha2::{Sha256, Sha512};

    #[test]
    fn base64_roundtrip() {
        let input = "Hello, World!";
        let encoded = STANDARD.encode(input.as_bytes());
        assert_eq!(encoded, "SGVsbG8sIFdvcmxkIQ==");
        let decoded = STANDARD.decode(&encoded).unwrap();
        assert_eq!(String::from_utf8(decoded).unwrap(), input);
    }

    #[test]
    fn url_encode_roundtrip() {
        let input = "hello world & foo=bar";
        let encoded = urlencoding::encode(input);
        let decoded = urlencoding::decode(&encoded).unwrap();
        assert_eq!(decoded, input);
    }

    #[test]
    fn sha256_known() {
        let mut h = Sha256::new();
        h.update(b"Hello, World!");
        let hex = format!("{:x}", h.finalize());
        assert_eq!(
            hex,
            "dffd6021bb2bd5b0af676290809ec3a53191dd81c7f70a4b28688a362182986f"
        );
    }

    #[test]
    fn sha512_known() {
        let mut h = Sha512::new();
        h.update(b"Hello, World!");
        let hex = format!("{:x}", h.finalize());
        assert!(hex.starts_with("374d794a95cdcfd8b35993"));
    }

    #[test]
    fn md5_known() {
        let mut h = Md5::new();
        h.update(b"Hello, World!");
        let hex = format!("{:x}", h.finalize());
        assert_eq!(hex, "65a8e27d8879283831b664bd8b7f0ad4");
    }
}

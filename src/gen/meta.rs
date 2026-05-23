use crate::error::Error;
use serde_json::json;

#[allow(clippy::too_many_arguments)]
pub fn run(
    title: &str,
    description: &str,
    url: Option<&str>,
    image: Option<&str>,
    page_type: &str,
    site_name: Option<&str>,
    author: Option<&str>,
    twitter_handle: Option<&str>,
    json_out: bool,
) -> Result<(), Error> {
    let mut tags: Vec<String> = Vec::new();

    // Basics
    tags.push(r#"<meta charset="UTF-8">"#.to_string());
    tags.push(format!("<title>{}</title>", esc(title)));
    tags.push(format!(
        r#"<meta name="description" content="{}">"#,
        esc(description)
    ));

    // Open Graph — order per spec: title, description, url, type, image, site_name.
    tags.push(format!(
        r#"<meta property="og:title" content="{}">"#,
        esc(title)
    ));
    tags.push(format!(
        r#"<meta property="og:description" content="{}">"#,
        esc(description)
    ));
    if let Some(u) = url {
        tags.push(format!(
            r#"<meta property="og:url" content="{}">"#,
            esc(u)
        ));
    }
    tags.push(format!(
        r#"<meta property="og:type" content="{}">"#,
        esc(page_type)
    ));
    if let Some(img) = image {
        tags.push(format!(
            r#"<meta property="og:image" content="{}">"#,
            esc(img)
        ));
    }
    if let Some(sn) = site_name {
        tags.push(format!(
            r#"<meta property="og:site_name" content="{}">"#,
            esc(sn)
        ));
    }

    // Twitter Card
    let twitter_card = if image.is_some() {
        "summary_large_image"
    } else {
        "summary"
    };
    tags.push(format!(
        r#"<meta name="twitter:card" content="{}">"#,
        twitter_card
    ));
    tags.push(format!(
        r#"<meta name="twitter:title" content="{}">"#,
        esc(title)
    ));
    tags.push(format!(
        r#"<meta name="twitter:description" content="{}">"#,
        esc(description)
    ));
    if let Some(img) = image {
        tags.push(format!(
            r#"<meta name="twitter:image" content="{}">"#,
            esc(img)
        ));
    }
    if let Some(handle) = twitter_handle {
        tags.push(format!(
            r#"<meta name="twitter:site" content="{}">"#,
            esc(handle)
        ));
    }
    if let Some(a) = author {
        if a.starts_with('@') {
            tags.push(format!(
                r#"<meta name="twitter:creator" content="{}">"#,
                esc(a)
            ));
        }
    }

    if json_out {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({ "tags": tags })).unwrap()
        );
    } else {
        for tag in &tags {
            println!("{}", tag);
        }
    }
    Ok(())
}

/// Escape HTML special characters in attribute/text content.
/// Order matters — `&` first, otherwise `&lt;` becomes `&amp;lt;`.
fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn esc_ampersand() {
        assert_eq!(esc("A & B"), "A &amp; B");
    }

    #[test]
    fn esc_quote() {
        assert_eq!(esc(r#"say "hi""#), "say &quot;hi&quot;");
    }

    #[test]
    fn esc_lt_gt() {
        assert_eq!(esc("<script>"), "&lt;script&gt;");
    }

    #[test]
    fn esc_order_is_safe() {
        // If `<` were escaped first, the resulting `&lt;` would be
        // double-escaped to `&amp;lt;` when `&` is processed.
        assert_eq!(esc("<&>"), "&lt;&amp;&gt;");
    }

    #[test]
    fn esc_unicode_passthrough() {
        // Unicode codepoints other than the four reserved chars are passed through.
        assert_eq!(esc("café — €"), "café — €");
    }
}

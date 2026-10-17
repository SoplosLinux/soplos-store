// AppStream descriptions are a small subset of HTML (<p>, <ul>, <li>,
// <em>, <code>). They are turned into plain paragraphs so the frontend
// never has to inject markup coming from a remote database.

pub fn html_paragraphs(html: &str) -> Vec<String> {
    let mut paragraphs = Vec::new();
    let mut current = String::new();
    let mut rest = html;
    while !rest.is_empty() {
        let Some(start) = rest.find('<') else {
            push_text(&mut current, rest);
            break;
        };
        push_text(&mut current, &rest[..start]);
        let Some(end) = rest[start..].find('>') else {
            break;
        };
        let tag = rest[start + 1..start + end].trim().to_lowercase();
        let closing = tag.starts_with('/');
        let name: String = tag.trim_start_matches('/').chars().take_while(|c| c.is_ascii_alphanumeric()).collect();
        if matches!(name.as_str(), "p" | "li" | "ul" | "ol" | "br") {
            flush(&mut paragraphs, &mut current);
            if name == "li" && !closing {
                current.push_str("• ");
            }
        }
        rest = &rest[start + end + 1..];
    }
    flush(&mut paragraphs, &mut current);
    paragraphs
}

/// Plain text descriptions (Snap Store) use blank lines between paragraphs.
pub fn text_paragraphs(text: &str) -> Vec<String> {
    text.split("\n\n").map(|p| p.split_whitespace().collect::<Vec<_>>().join(" ")).filter(|p| !p.is_empty()).collect()
}

fn push_text(current: &mut String, text: &str) {
    let decoded = text
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&apos;", "'")
        .replace("&amp;", "&");
    for word in decoded.split_whitespace() {
        if !current.is_empty() && !current.ends_with(' ') {
            current.push(' ');
        }
        current.push_str(word);
    }
}

fn flush(paragraphs: &mut Vec<String>, current: &mut String) {
    let text = current.trim();
    if !text.is_empty() && text != "•" {
        paragraphs.push(text.to_string());
    }
    current.clear();
}

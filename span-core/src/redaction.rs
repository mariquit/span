use regex::Regex;
use std::sync::OnceLock;

pub(crate) fn redact(input: &str) -> (String, bool) {
    static PATTERNS: OnceLock<Vec<(Regex, &'static str)>> = OnceLock::new();
    let patterns = PATTERNS.get_or_init(|| vec![
        (Regex::new(r"(?i)(authorization\s*:\s*(?:bearer|basic)\s+)[^\s,;]+").unwrap(), "$1[REDACTED:authorization]"),
        (Regex::new(r#"(?i)\b(api[_-]?key|access[_-]?token|secret|password|passwd|token)\b\s*[:=]\s*['"]?[^\s,'";]+"#).unwrap(), "$1=[REDACTED:credential]"),
        (Regex::new(r"\beyJ[A-Za-z0-9_-]{8,}\.[A-Za-z0-9_-]{8,}\.[A-Za-z0-9_-]{8,}\b").unwrap(), "[REDACTED:jwt]"),
        (Regex::new(r"-----BEGIN (?:RSA |EC |OPENSSH |DSA |PGP )?PRIVATE KEY-----[\s\S]*?-----END (?:RSA |EC |OPENSSH |DSA |PGP )?PRIVATE KEY-----").unwrap(), "[REDACTED:private_key]"),
        (Regex::new(r"\b(?:sk|ghp|github_pat|ghs|ghu|ghr|gho)_[A-Za-z0-9_-]{16,}\b").unwrap(), "[REDACTED:token]"),
        (Regex::new(r"\b(?:sk-ant|sk-proj|sk-admin)-[A-Za-z0-9_-]{16,}\b").unwrap(), "[REDACTED:anthropic_key]"),
        (Regex::new(r"\bAIza[0-9A-Za-z_-]{35}\b").unwrap(), "[REDACTED:google_api_key]"),
        (Regex::new(r"\bya29\.[A-Za-z0-9_-]{20,}\b").unwrap(), "[REDACTED:google_oauth_token]"),
        (Regex::new(r"\bxai-[A-Za-z0-9_-]{20,}\b").unwrap(), "[REDACTED:xai_key]"),
        (Regex::new(r"\bgsk_[A-Za-z0-9_-]{20,}\b").unwrap(), "[REDACTED:groq_key]"),
        (Regex::new(r"\bhf_[A-Za-z0-9_-]{20,}\b").unwrap(), "[REDACTED:huggingface_token]"),
        (Regex::new(r"\br8_[A-Za-z0-9_-]{20,}\b").unwrap(), "[REDACTED:replicate_token]"),
        (Regex::new(r"\bpplx-[A-Za-z0-9_-]{20,}\b").unwrap(), "[REDACTED:perplexity_key]"),
        (Regex::new(r"\bdapi[0-9a-fA-F]{32}\b").unwrap(), "[REDACTED:databricks_token]"),
        (Regex::new(r"\bAKIA[0-9A-Z]{16}\b").unwrap(), "[REDACTED:aws_access_key]"),
        (Regex::new(r"\bASIA[0-9A-Z]{16}\b").unwrap(), "[REDACTED:aws_session_token]"),
        (Regex::new(r"\bdop_v1_[0-9a-f]{64}\b").unwrap(), "[REDACTED:digitalocean_token]"),
        (Regex::new(r"\bglpat-[A-Za-z0-9_-]{20,}\b").unwrap(), "[REDACTED:gitlab_pat]"),
        (Regex::new(r"\bnpm_[A-Za-z0-9_-]{20,}\b").unwrap(), "[REDACTED:npm_token]"),
        (Regex::new(r"\bpypi-AgEI[A-Za-z0-9_-]{20,}\b").unwrap(), "[REDACTED:pypi_token]"),
        (Regex::new(r"\bsk_(?:live|test)_[A-Za-z0-9]{16,}\b").unwrap(), "[REDACTED:stripe_secret]"),
        (Regex::new(r"\brk_(?:live|test)_[A-Za-z0-9]{16,}\b").unwrap(), "[REDACTED:stripe_restricted]"),
        (Regex::new(r"\bsq0[a-z]{3}-[A-Za-z0-9_-]{20,}\b").unwrap(), "[REDACTED:square_token]"),
        (Regex::new(r"\baccess_token\$production\$[A-Za-z0-9_-]{20,}\b").unwrap(), "[REDACTED:braintree_token]"),
        (Regex::new(r"\bxox[abpr]-[A-Za-z0-9-]{10,}\b").unwrap(), "[REDACTED:slack_token]"),
        (Regex::new(r"https://hooks\.slack\.com/services/[A-Za-z0-9/]+").unwrap(), "[REDACTED:slack_webhook]"),
        (Regex::new(r"\bSK[0-9a-fA-F]{32}\b").unwrap(), "[REDACTED:twilio_api_key]"),
        (Regex::new(r"\bAC[0-9a-fA-F]{32}\b").unwrap(), "[REDACTED:twilio_account_sid]"),
        (Regex::new(r"\bSG\.[A-Za-z0-9_-]{22}\.[A-Za-z0-9_-]{43}\b").unwrap(), "[REDACTED:sendgrid_key]"),
        (Regex::new(r"\bkey-[A-Za-z0-9]{32}\b").unwrap(), "[REDACTED:mailgun_key]"),
        (Regex::new(r"\bshpat_[0-9a-fA-F]{32}\b").unwrap(), "[REDACTED:shopify_token]"),
        (Regex::new(r"(?i)\b(?:postgres(?:ql)?|mysql|mongodb(?:\+srv)?)://[^\s:@]+:[^\s@]+@[^\s]+").unwrap(), "[REDACTED:db_url]"),
        (Regex::new(r"\b\d{3}-\d{2}-\d{4}\b").unwrap(), "[REDACTED:ssn]"),
    ]);
    let mut value = input.to_owned();
    for (pattern, replacement) in patterns {
        value = pattern.replace_all(&value, *replacement).into_owned();
    }
    let changed = value != input;
    (value, changed)
}

/// Sanitize every untrusted text field before persistence.
pub(crate) fn sanitize(input: &str) -> (String, bool) {
    let mut output = String::new();
    let mut depth = 0usize;
    let mut rest = input;
    while !rest.is_empty() {
        if rest.starts_with('<') {
            if let Some(end) = rest.find('>') {
                let tag = rest[1..end].trim().to_ascii_lowercase();
                let closing = tag.starts_with('/');
                let name = tag
                    .trim_start_matches('/')
                    .split_whitespace()
                    .next()
                    .unwrap_or("");
                if name == "private" {
                    if closing {
                        depth = depth.saturating_sub(1);
                    } else {
                        depth += 1;
                    }
                    rest = &rest[end + 1..];
                    continue;
                }
            } else if rest.to_ascii_lowercase().starts_with("<private") {
                break;
            }
        }
        let ch = rest.chars().next().unwrap();
        if depth == 0 {
            output.push(ch);
        }
        rest = &rest[ch.len_utf8()..];
    }
    let (clean, redacted) = redact(&output);
    let changed = redacted || output != input;
    (clean, changed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redacts_common_secrets() {
        let (value, changed) = redact("Authorization: Bearer abcdef password=hunter2");
        assert!(changed);
        assert!(!value.contains("abcdef"));
        assert!(!value.contains("hunter2"));
    }

    #[test]
    fn redacts_jwt_and_db_url() {
        let (value, changed) =
            redact("token=eyJaaaaaaa.bbbbbbbb.cccccccc postgres://user:pass@host/db");
        assert!(changed);
        assert!(!value.contains("pass@host"));
    }
}

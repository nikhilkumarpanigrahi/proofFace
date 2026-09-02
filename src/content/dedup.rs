use std::collections::HashSet;
use url::Url;

pub struct CandidateDeduplicator {
    seen_urls: HashSet<String>,
}

impl CandidateDeduplicator {
    pub fn new() -> Self {
        Self {
            seen_urls: HashSet::new(),
        }
    }

    /// Normalizes a URL by lowercasing scheme/host, stripping tracking query params and fragments.
    pub fn normalize_url(raw_url: &str) -> String {
        if let Ok(mut parsed) = Url::parse(raw_url) {
            parsed.set_fragment(None);

            // Filter out tracking parameters like utm_*, fbclid, ref, etc.
            let clean_pairs: Vec<(String, String)> = parsed
                .query_pairs()
                .filter(|(k, _)| {
                    let key = k.to_lowercase();
                    !key.starts_with("utm_")
                        && key != "fbclid"
                        && key != "gclid"
                        && key != "ref"
                        && key != "source"
                })
                .map(|(k, v)| (k.into_owned(), v.into_owned()))
                .collect();

            if clean_pairs.is_empty() {
                parsed.set_query(None);
            } else {
                let mut serializer = url::form_urlencoded::Serializer::new(String::new());
                for (k, v) in clean_pairs {
                    serializer.append_pair(&k, &v);
                }
                parsed.set_query(Some(&serializer.finish()));
            }

            parsed.to_string()
        } else {
            raw_url.trim().to_string()
        }
    }

    /// Returns true if the URL is unique (not seen before) and records it.
    pub fn check_and_insert(&mut self, raw_url: &str) -> bool {
        let normalized = Self::normalize_url(raw_url);
        self.seen_urls.insert(normalized)
    }
}

impl Default for CandidateDeduplicator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_url_normalization_strips_tracking() {
        let u1 = "https://example.com/article?utm_source=twitter&utm_medium=social&id=123#header";
        let norm = CandidateDeduplicator::normalize_url(u1);
        assert_eq!(norm, "https://example.com/article?id=123");
    }

    #[test]
    fn test_deduplicator_filters_duplicates() {
        let mut dedup = CandidateDeduplicator::new();
        let u1 = "https://example.com/item/1?utm_source=google";
        let u2 = "https://example.com/item/1";
        let u3 = "https://example.com/item/2";

        assert!(dedup.check_and_insert(u1));
        assert!(!dedup.check_and_insert(u2)); // duplicate after normalization
        assert!(dedup.check_and_insert(u3)); // unique
    }
}

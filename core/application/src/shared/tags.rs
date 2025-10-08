use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TagSet(BTreeMap<String, String>);

impl TagSet {
    pub fn new<I, K, V>(pairs: I) -> Self
    where
        I: IntoIterator<Item = (K, V)>,
        K: Into<String>,
        V: Into<String>,
    {
        let mut map = BTreeMap::new();
        for (key, value) in pairs {
            let key = key.into().trim().to_ascii_lowercase();
            let value = value.into().trim().to_string();
            map.insert(key, value);
        }
        Self(map)
    }

    pub fn canonical_fingerprint(&self) -> String {
        self.0
            .iter()
            .map(|(key, value)| format!("{key}={value}"))
            .collect::<Vec<_>>()
            .join(";")
    }
}

impl Default for TagSet {
    fn default() -> Self {
        Self(BTreeMap::new())
    }
}

#[cfg(test)]
mod tests {
    use super::TagSet;

    #[test]
    fn canonicalizes_keys_and_orders_them() {
        let tag_set = TagSet::new([(" Metric ", " value "), ("Source", "Fitbit")]);

        assert_eq!(
            tag_set.canonical_fingerprint(),
            "metric=value;source=Fitbit"
        );
    }
}

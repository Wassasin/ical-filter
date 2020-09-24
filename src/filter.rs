use regex::Regex;
use serde::de;
use std::fmt;

type MyError = Box<dyn std::error::Error>;

pub const TRUE_FILTER: Filter = Filter {
    invert: false,
    operator: FilterOperator::True,
};

#[derive(Debug)]
enum FilterOperator {
    Equals(String),
    StartsWith(String),
    EndsWith(String),
    Contains(String),
    True,
    Regex(Regex),
}

#[derive(Debug)]
pub struct Filter {
    invert: bool,
    operator: FilterOperator,
}

impl FilterOperator {
    /// Parses a stringified filter operator into a [`FilterOperator`](enum.FilterOperator.html)
    fn parse(s: &str, content: String) -> Result<FilterOperator, MyError> {
        match s {
            "equals" => Ok(FilterOperator::Equals(content)),
            "startsWith" => Ok(FilterOperator::StartsWith(content)),
            "endsWith" => Ok(FilterOperator::EndsWith(content)),
            "contains" => Ok(FilterOperator::Contains(content)),
            "true" => Ok(FilterOperator::True),
            "regex" => Ok(FilterOperator::Regex(Regex::new(&content)?)),
            _ => Err("unknown filter operator; options are equals, startsWith, endsWith, contains, true, regex".into()),
        }
    }

    /// Execute the filter operator on the given string. Returns whether it
    /// matches.
    fn matches(&self, s: &str) -> bool {
        match self {
            FilterOperator::Equals(pat) => s == pat,
            FilterOperator::StartsWith(pat) => s.starts_with(pat),
            FilterOperator::EndsWith(pat) => s.ends_with(pat),
            FilterOperator::Contains(pat) => s.contains(pat),
            FilterOperator::True => true,
            FilterOperator::Regex(pat) => pat.is_match(s),
        }
    }
}

impl Filter {
    /// Parses a filter into a Filter struct.
    fn parse(s: &str) -> Result<Filter, MyError> {
        let invert = s.starts_with('!');
        let s = if invert { &s[1..] } else { s };

        let colon = s.find(":").ok_or_else(|| "missing colon in filter")?;
        let (operator, text) = s.split_at(colon);
        // chop off colon
        let text = &text[1..];
        let operator = FilterOperator::parse(operator, text.to_owned())?;
        Ok(Filter { invert, operator })
    }

    /// Execute the filter on the given string. Returns whether it matches.
    pub fn matches(&self, s: &str) -> bool {
        let result = self.operator.matches(s);
        if self.invert {
            !result
        } else {
            result
        }
    }
}

pub fn deserialize_filter_list<'de, D>(ds: D) -> Result<Option<Vec<Filter>>, D::Error>
where
    D: de::Deserializer<'de>,
{
    // inspired by https://github.com/actix/actix-web/issues/1301#issuecomment-687041548
    struct StringVecVisitor;

    impl<'de> de::Visitor<'de> for StringVecVisitor {
        type Value = Option<Vec<Filter>>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            // tilde arbitrarily chosen as it is very unlikely to appear in a filter
            formatter.write_str("a tilde separated list of [!]operator:filter")
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            let mut filters = Vec::new();
            for filt in v.split("~") {
                let parsed = Filter::parse(filt).map_err(E::custom)?;
                filters.push(parsed);
            }

            Ok(Some(filters))
        }
    }

    ds.deserialize_any(StringVecVisitor)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_load_filter() {
        let qs = "startsWith:aaa";
        let parsed = Filter::parse(qs).unwrap();
        assert_eq!(parsed.invert, false);
        assert!(matches!(
            parsed.operator,
            FilterOperator::StartsWith(a) if a == "aaa"
        ));

        let qs = "true:aaa";
        let parsed = Filter::parse(qs).unwrap();
        assert_eq!(parsed.invert, false);
        assert!(matches!(parsed.operator, FilterOperator::True));
        assert!(parsed.matches("a"));

        let qs = "!true:aaa";
        let parsed = Filter::parse(qs).unwrap();
        assert_eq!(parsed.invert, true);
        assert!(matches!(parsed.operator, FilterOperator::True));
        assert!(!parsed.matches("a"));
    }
}

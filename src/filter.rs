use regex::Regex;
use serde::de;
use std::fmt;

// this lint is falsely triggering on this, which is *not* interior mutable
#[allow(clippy::declare_interior_mutable_const)]
pub const TRUE_FILTER: Filter = Filter {
    invert: false,
    operator: FilterOperator::True,
};

#[derive(Debug)]
pub enum FilterErrorKind {
    MissingColon,
    OperatorParse,
    RegexError(regex::Error),
}

#[derive(Debug)]
pub struct FilterError(String, FilterErrorKind);

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
    fn parse(s: &str, content: String) -> Result<FilterOperator, FilterError> {
        match s {
            "equals" => Ok(FilterOperator::Equals(content)),
            "startsWith" => Ok(FilterOperator::StartsWith(content)),
            "endsWith" => Ok(FilterOperator::EndsWith(content)),
            "contains" => Ok(FilterOperator::Contains(content)),
            "true" => Ok(FilterOperator::True),
            "regex" => Ok(FilterOperator::Regex(Regex::new(&content).map_err(|e| FilterError(content, FilterErrorKind::RegexError(e)))?)),
            _ => Err(FilterError("unknown filter operator; options are equals, startsWith, endsWith, contains, true, regex".to_owned(), FilterErrorKind::OperatorParse)),
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
    fn parse(s: &str) -> Result<Filter, FilterError> {
        let invert = s.starts_with('!');
        let s = if invert { &s[1..] } else { s };

        let colon = s
            .find(':')
            .ok_or_else(|| FilterError(s.to_owned(), FilterErrorKind::MissingColon))?;
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

impl<'de> de::Deserialize<'de> for Filter {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct FilterVisitor;

        impl<'de> de::Visitor<'de> for FilterVisitor {
            type Value = Filter;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("[!]operator:filter")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Filter::parse(v).map_err(|e| E::custom(format!("{:?}", e)))
            }
        }

        deserializer.deserialize_any(FilterVisitor)
    }
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

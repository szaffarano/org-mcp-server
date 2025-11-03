//! Date placeholder replacement utilities for test fixtures.
//!
//! This module provides functions to replace date placeholders in org-mode test fixtures
//! with actual dates, allowing tests to work with dynamic dates relative to the test execution time.

use chrono::{Datelike, Days, NaiveDate, Weekday};
use once_cell::sync::Lazy;
use regex::Regex;

static DATE_PLACEHOLDER_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"<(@[A-Z_+\-0-9]+@)\s+[A-Za-z]{3}(\s+\d{1,2}:\d{2})?>").unwrap());

/// Format a date in org-mode format: <YYYY-MM-DD DayOfWeek>
pub fn format_org_date(date: NaiveDate, include_time: Option<&str>) -> String {
    let day_name = match date.weekday() {
        Weekday::Mon => "Mon",
        Weekday::Tue => "Tue",
        Weekday::Wed => "Wed",
        Weekday::Thu => "Thu",
        Weekday::Fri => "Fri",
        Weekday::Sat => "Sat",
        Weekday::Sun => "Sun",
    };

    if let Some(time) = include_time {
        format!("<{} {} {}>", date.format("%Y-%m-%d"), day_name, time)
    } else {
        format!("<{} {}>", date.format("%Y-%m-%d"), day_name)
    }
}

/// Get the start of the current week (Monday)
pub fn week_start(base_date: NaiveDate) -> NaiveDate {
    let weekday = base_date.weekday().num_days_from_monday();
    base_date
        .checked_sub_days(Days::new(weekday as u64))
        .unwrap_or(base_date)
}

/// Get the end of the current week (Sunday)
pub fn week_end(base_date: NaiveDate) -> NaiveDate {
    let weekday = base_date.weekday().num_days_from_monday();
    let days_until_sunday = 6 - weekday;
    base_date
        .checked_add_days(Days::new(days_until_sunday as u64))
        .unwrap_or(base_date)
}

/// Replace date placeholders in content with actual dates relative to base_date
///
/// Supported placeholders:
/// - `@TODAY@` - Current date
/// - `@TODAY+N@` - N days from today (e.g., `@TODAY+7@`)
/// - `@TODAY-N@` - N days before today (e.g., `@TODAY-1@`)
/// - `@WEEK_START@` - Start of current week (Monday)
/// - `@WEEK_END@` - End of current week (Sunday)
///
/// The placeholder should be in an org-mode timestamp, optionally with time:
/// - `<@TODAY@ Mon>` → `<2025-11-02 Sat>`
/// - `<@TODAY+1@ Tue 10:00>` → `<2025-11-03 Sun 10:00>`
pub fn replace_dates_in_content(content: &str, base_date: NaiveDate) -> String {
    // Regex to match org-mode timestamps with placeholders
    // Matches: <@PLACEHOLDER@ [DayOfWeek] [Time]>
    // let re = Regex::new(r"<(@[A-Z_+\-0-9]+@)\s+[A-Za-z]{3}(\s+\d{1,2}:\d{2})?>").unwrap();
    DATE_PLACEHOLDER_REGEX
        .replace_all(content, |caps: &regex::Captures| {
            let placeholder = &caps[1];
            let time = caps.get(2).map(|m| m.as_str().trim());

            let date = parse_placeholder(placeholder, base_date);
            format_org_date(date, time)
        })
        .to_string()
}

/// Parse a placeholder string to get the corresponding date
fn parse_placeholder(placeholder: &str, base_date: NaiveDate) -> NaiveDate {
    match placeholder {
        "@TODAY@" => base_date,
        "@WEEK_START@" => week_start(base_date),
        "@WEEK_END@" => week_end(base_date),
        _ if placeholder.starts_with("@TODAY+") && placeholder.ends_with('@') => {
            // Extract the number from @TODAY+N@
            let num_str = &placeholder[7..placeholder.len() - 1];
            if let Ok(days) = num_str.parse::<u64>() {
                base_date
                    .checked_add_days(Days::new(days))
                    .unwrap_or(base_date)
            } else {
                base_date
            }
        }
        _ if placeholder.starts_with("@TODAY-") && placeholder.ends_with('@') => {
            // Extract the number from @TODAY-N@
            let num_str = &placeholder[7..placeholder.len() - 1];
            if let Ok(days) = num_str.parse::<u64>() {
                base_date
                    .checked_sub_days(Days::new(days))
                    .unwrap_or(base_date)
            } else {
                base_date
            }
        }
        _ => base_date, // Unknown placeholder, return base_date
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_org_date_without_time() {
        let date = NaiveDate::from_ymd_opt(2025, 11, 2).unwrap();
        let formatted = format_org_date(date, None);
        assert_eq!(formatted, "<2025-11-02 Sun>");
    }

    #[test]
    fn test_format_org_date_with_time() {
        let date = NaiveDate::from_ymd_opt(2025, 11, 2).unwrap();
        let formatted = format_org_date(date, Some("10:00"));
        assert_eq!(formatted, "<2025-11-02 Sun 10:00>");
    }

    #[test]
    fn test_replace_dates_today() {
        let base = NaiveDate::from_ymd_opt(2025, 11, 2).unwrap();
        let content = "* TODO Task\nSCHEDULED: <@TODAY@ Sun>";
        let result = replace_dates_in_content(content, base);
        assert!(result.contains("<2025-11-02 Sun>"));
    }

    #[test]
    fn test_replace_dates_today_plus() {
        let base = NaiveDate::from_ymd_opt(2025, 11, 2).unwrap();
        let content = "* TODO Task\nDEADLINE: <@TODAY+7@ Sun>";
        let result = replace_dates_in_content(content, base);
        assert!(result.contains("<2025-11-09 Sun>"));
    }

    #[test]
    fn test_replace_dates_with_time() {
        let base = NaiveDate::from_ymd_opt(2025, 11, 2).unwrap();
        let content = "* TODO Meeting\nSCHEDULED: <@TODAY@ Sun 10:00>";
        let result = replace_dates_in_content(content, base);
        assert!(result.contains("<2025-11-02 Sun 10:00>"));
    }
}

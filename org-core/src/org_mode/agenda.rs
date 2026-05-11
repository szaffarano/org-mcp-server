use chrono::{DateTime, Datelike, Days, Duration, Local, Months, NaiveDate, TimeZone};

use crate::OrgModeError;
use crate::org_mode::{AgendaViewType, OrgMode};

impl OrgMode {
    pub(crate) fn to_start_of_day(date: DateTime<Local>) -> DateTime<Local> {
        date.date_naive()
            .and_hms_opt(0, 0, 0)
            .and_then(|dt| match Local.from_local_datetime(&dt) {
                chrono::LocalResult::Single(t) => Some(t),
                chrono::LocalResult::Ambiguous(t, _) => Some(t),
                chrono::LocalResult::None => {
                    let dt_plus_1 = dt + chrono::Duration::hours(1);
                    match Local.from_local_datetime(&dt_plus_1) {
                        chrono::LocalResult::Single(t) => Some(t),
                        chrono::LocalResult::Ambiguous(t, _) => Some(t),
                        chrono::LocalResult::None => None,
                    }
                }
            })
            .unwrap_or(date)
    }

    pub(crate) fn to_end_of_day(date: DateTime<Local>) -> DateTime<Local> {
        date.date_naive()
            .and_hms_opt(23, 59, 59)
            .and_then(|dt| match Local.from_local_datetime(&dt) {
                chrono::LocalResult::Single(t) => Some(t),
                chrono::LocalResult::Ambiguous(t, _) => Some(t),
                chrono::LocalResult::None => {
                    let dt_minus_1 = dt - chrono::Duration::hours(1);
                    match Local.from_local_datetime(&dt_minus_1) {
                        chrono::LocalResult::Single(t) => Some(t),
                        chrono::LocalResult::Ambiguous(t, _) => Some(t),
                        chrono::LocalResult::None => None,
                    }
                }
            })
            .unwrap_or(date)
    }

    pub(crate) fn naive_date_to_local(
        date: NaiveDate,
        hour: u32,
        min: u32,
        sec: u32,
    ) -> Result<DateTime<Local>, OrgModeError> {
        date.and_hms_opt(hour, min, sec)
            .and_then(|dt| match Local.from_local_datetime(&dt) {
                chrono::LocalResult::Single(t) => Some(t),
                chrono::LocalResult::Ambiguous(t, _) => Some(t),
                chrono::LocalResult::None => {
                    let dt_plus_1 = dt + chrono::Duration::hours(1);
                    match Local.from_local_datetime(&dt_plus_1) {
                        chrono::LocalResult::Single(t) => Some(t),
                        chrono::LocalResult::Ambiguous(t, _) => Some(t),
                        chrono::LocalResult::None => None,
                    }
                }
            })
            .ok_or_else(|| {
                OrgModeError::InvalidAgendaViewType(format!(
                    "Could not convert date '{}' to local timezone",
                    date
                ))
            })
    }

    pub(crate) fn last_day_of_month(date: DateTime<Local>) -> DateTime<Local> {
        let month = date.month();
        let year = date.year();

        let (next_month, next_year) = if month == 12 {
            (1, year + 1)
        } else {
            (month + 1, year)
        };

        let next_month_first = Self::to_start_of_day(
            date.with_year(next_year)
                .unwrap()
                .with_day(1)
                .unwrap()
                .with_month(next_month)
                .unwrap(),
        );

        next_month_first - Duration::days(1)
    }

    pub(crate) fn add_repeater_duration(
        date: DateTime<Local>,
        value: u64,
        unit: &orgize::ast::TimeUnit,
    ) -> DateTime<Local> {
        match unit {
            orgize::ast::TimeUnit::Hour => Some(date + Duration::hours(value as i64)),
            orgize::ast::TimeUnit::Day => date.checked_add_days(Days::new(value)),
            orgize::ast::TimeUnit::Week => date.checked_add_days(Days::new(value * 7)),
            orgize::ast::TimeUnit::Month => date.checked_add_months(Months::new(value as u32)),
            orgize::ast::TimeUnit::Year => date.checked_add_months(Months::new(value as u32 * 12)),
        }
        .unwrap_or(date)
    }

    pub(crate) fn parse_date_string(
        date_str: &str,
        context: &str,
    ) -> Result<NaiveDate, OrgModeError> {
        NaiveDate::parse_from_str(date_str, "%Y-%m-%d").map_err(|_| {
            OrgModeError::InvalidAgendaViewType(format!(
                "Invalid {context} '{date_str}', expected YYYY-MM-DD"
            ))
        })
    }
}

impl AgendaViewType {
    pub fn start_date(&self) -> DateTime<Local> {
        let date = match self {
            AgendaViewType::Today => Local::now(),
            AgendaViewType::Day(d) => *d,
            AgendaViewType::CurrentWeek => {
                let now = Local::now();
                let weekday = now.weekday().num_days_from_monday();
                now - Duration::days(weekday as i64)
            }
            AgendaViewType::Week(week_num) => {
                let now = Local::now();
                let year_start =
                    OrgMode::to_start_of_day(now.with_month(1).unwrap().with_day(1).unwrap());
                year_start + Duration::weeks(*week_num as i64)
            }
            AgendaViewType::CurrentMonth => {
                let now = Local::now();
                now.with_day(1).unwrap()
            }
            AgendaViewType::Month(month) => {
                let now = Local::now().with_day(1).unwrap_or(Local::now());
                now.with_month(*month).unwrap_or(now).with_day(1).unwrap()
            }
            AgendaViewType::Custom { from, .. } => *from,
        };
        OrgMode::to_start_of_day(date)
    }

    pub fn end_date(&self) -> DateTime<Local> {
        let date = match self {
            AgendaViewType::Today => Local::now(),
            AgendaViewType::Day(d) => *d,
            AgendaViewType::CurrentWeek => {
                let now = Local::now();
                let weekday = now.weekday().num_days_from_monday();
                let start = now - Duration::days(weekday as i64);
                start + Duration::days(6)
            }
            AgendaViewType::Week(week_num) => {
                let now = Local::now();
                let year_start =
                    OrgMode::to_start_of_day(now.with_month(1).unwrap().with_day(1).unwrap());
                let target_week_start = year_start + Duration::weeks(*week_num as i64);
                target_week_start + Duration::days(6)
            }
            AgendaViewType::CurrentMonth => {
                let now = Local::now();
                OrgMode::last_day_of_month(now)
            }
            AgendaViewType::Month(month) => {
                let now = Local::now().with_day(1).unwrap_or(Local::now());
                let date_in_month = now.with_month(*month).unwrap_or(now);
                OrgMode::last_day_of_month(date_in_month)
            }
            AgendaViewType::Custom { to, .. } => *to,
        };
        OrgMode::to_end_of_day(date)
    }
}

impl TryFrom<&str> for AgendaViewType {
    type Error = OrgModeError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        if value.is_empty() {
            return Ok(AgendaViewType::default());
        }

        match value {
            "today" => Ok(AgendaViewType::Today),
            "week" => Ok(AgendaViewType::CurrentWeek),
            "month" => Ok(AgendaViewType::CurrentMonth),
            _ => {
                let parts: Vec<&str> = value.split('/').collect();

                match parts.as_slice() {
                    ["day", date_str] => {
                        let parsed_date = OrgMode::parse_date_string(date_str, "date format")?;
                        let datetime = OrgMode::naive_date_to_local(parsed_date, 0, 0, 0)?;
                        Ok(AgendaViewType::Day(datetime))
                    }
                    ["week", week_str] => {
                        let week_num = week_str.parse::<u8>().map_err(|_| {
                            OrgModeError::InvalidAgendaViewType(format!(
                                "Invalid week number '{}', expected 0-53",
                                week_str
                            ))
                        })?;
                        if week_num > 53 {
                            return Err(OrgModeError::InvalidAgendaViewType(format!(
                                "Week number {} out of range, expected 0-53",
                                week_num
                            )));
                        }
                        Ok(AgendaViewType::Week(week_num))
                    }
                    ["month", month_str] => {
                        let month_num = month_str.parse::<u32>().map_err(|_| {
                            OrgModeError::InvalidAgendaViewType(format!(
                                "Invalid month number '{}', expected 1-12",
                                month_str
                            ))
                        })?;
                        if !(1..=12).contains(&month_num) {
                            return Err(OrgModeError::InvalidAgendaViewType(format!(
                                "Month number {} out of range, expected 1-12",
                                month_num
                            )));
                        }
                        Ok(AgendaViewType::Month(month_num))
                    }
                    ["query", "from", from_str, "to", to_str] => {
                        let from_date = OrgMode::parse_date_string(from_str, "from date")?;
                        let to_date = OrgMode::parse_date_string(to_str, "to date")?;

                        let from_datetime = OrgMode::naive_date_to_local(from_date, 0, 0, 0)?;
                        let to_datetime = OrgMode::naive_date_to_local(to_date, 23, 59, 59)?;

                        if from_datetime > to_datetime {
                            return Err(OrgModeError::InvalidAgendaViewType(
                                "From date must be before to date".into(),
                            ));
                        }
                        Ok(AgendaViewType::Custom {
                            from: from_datetime,
                            to: to_datetime,
                        })
                    }
                    _ => Err(OrgModeError::InvalidAgendaViewType(format!(
                        "Unknown agenda view type format: '{}'",
                        value
                    ))),
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Timelike;
    use orgize::ast::TimeUnit;

    #[test]
    fn test_add_repeater_duration_hour() {
        let date = Local.with_ymd_and_hms(2025, 6, 15, 14, 0, 0).unwrap();
        let result = OrgMode::add_repeater_duration(date, 2, &TimeUnit::Hour);

        assert_eq!(result.hour(), 16);
        assert_eq!(result.day(), 15);
    }

    #[test]
    fn test_add_repeater_duration_day() {
        let date = Local.with_ymd_and_hms(2025, 6, 15, 12, 0, 0).unwrap();
        let result = OrgMode::add_repeater_duration(date, 5, &TimeUnit::Day);

        assert_eq!(result.day(), 20);
        assert_eq!(result.month(), 6);
    }

    #[test]
    fn test_add_repeater_duration_week() {
        let date = Local.with_ymd_and_hms(2025, 6, 15, 12, 0, 0).unwrap();
        let result = OrgMode::add_repeater_duration(date, 2, &TimeUnit::Week);

        assert_eq!(result.day(), 29);
        assert_eq!(result.month(), 6);
    }

    #[test]
    fn test_add_repeater_duration_month() {
        let date = Local.with_ymd_and_hms(2025, 6, 15, 12, 0, 0).unwrap();
        let result = OrgMode::add_repeater_duration(date, 3, &TimeUnit::Month);

        assert_eq!(result.month(), 9);
        assert_eq!(result.day(), 15);
        assert_eq!(result.year(), 2025);
    }

    #[test]
    fn test_add_repeater_duration_year() {
        let date = Local.with_ymd_and_hms(2025, 6, 15, 12, 0, 0).unwrap();
        let result = OrgMode::add_repeater_duration(date, 2, &TimeUnit::Year);

        assert_eq!(result.year(), 2027);
        assert_eq!(result.month(), 6);
        assert_eq!(result.day(), 15);
    }

    #[test]
    fn test_add_repeater_duration_month_boundary() {
        let date = Local.with_ymd_and_hms(2025, 10, 15, 12, 0, 0).unwrap();
        let result = OrgMode::add_repeater_duration(date, 3, &TimeUnit::Month);

        assert_eq!(result.year(), 2026);
        assert_eq!(result.month(), 1);
        assert_eq!(result.day(), 15);
    }

    #[test]
    fn test_last_day_of_month_from_day_31() {
        let date = Local.with_ymd_and_hms(2025, 1, 31, 12, 0, 0).unwrap();
        let result = OrgMode::last_day_of_month(date);

        assert_eq!(result.month(), 1);
        assert_eq!(result.day(), 31);
    }

    #[test]
    fn test_last_day_of_month_february() {
        let date = Local.with_ymd_and_hms(2025, 2, 15, 12, 0, 0).unwrap();
        let result = OrgMode::last_day_of_month(date);

        assert_eq!(result.month(), 2);
        assert_eq!(result.day(), 28);
    }

    #[test]
    fn test_last_day_of_month_leap_year() {
        let date = Local.with_ymd_and_hms(2024, 2, 15, 12, 0, 0).unwrap();
        let result = OrgMode::last_day_of_month(date);

        assert_eq!(result.month(), 2);
        assert_eq!(result.day(), 29);
    }
}

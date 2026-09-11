use exchange_types::{
    ErrorCode, Market, QueryError, TimeUnit, Timestamp, TradeAnchor, TradeQuery, TradeWindow,
    format_beijing, format_utc, parse_time,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AnchorMode {
    Id,
    Time,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TimeUnitChoice {
    Auto,
    Seconds,
    Millis,
    Micros,
}

impl TimeUnitChoice {
    pub const fn explicit(self) -> Option<TimeUnit> {
        match self {
            Self::Auto => None,
            Self::Seconds => Some(TimeUnit::Seconds),
            Self::Millis => Some(TimeUnit::Millis),
            Self::Micros => Some(TimeUnit::Micros),
        }
    }

    pub const fn value(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Seconds => "s",
            Self::Millis => "ms",
            Self::Micros => "us",
        }
    }

    pub fn from_value(value: &str) -> Self {
        match value {
            "s" => Self::Seconds,
            "ms" => Self::Millis,
            "us" => Self::Micros,
            _ => Self::Auto,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CountField {
    Before,
    After,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct QueryForm {
    pub symbol: String,
    pub anchor_mode: AnchorMode,
    pub anchor: String,
    pub time_unit: TimeUnitChoice,
    pub before: String,
    pub after: String,
}

impl Default for QueryForm {
    fn default() -> Self {
        Self {
            symbol: "BTCUSDT".into(),
            anchor_mode: AnchorMode::Id,
            anchor: String::new(),
            time_unit: TimeUnitChoice::Auto,
            before: "50".into(),
            after: "50".into(),
        }
    }
}

impl QueryForm {
    pub fn parse(&self, market: Market) -> Result<TradeQuery, QueryError> {
        let before = parse_count("前序数量", &self.before)?;
        let after = parse_count("后序数量", &self.after)?;
        let anchor = match self.anchor_mode {
            AnchorMode::Id => {
                if self.anchor.is_empty() || !self.anchor.bytes().all(|byte| byte.is_ascii_digit())
                {
                    return Err(invalid("成交 ID 只能包含数字"));
                }
                TradeAnchor::Id(
                    self.anchor
                        .parse()
                        .map_err(|_| invalid("成交 ID 超出 u64 范围"))?,
                )
            }
            AnchorMode::Time => {
                TradeAnchor::Time(parse_time(self.anchor.trim(), self.time_unit.explicit())?)
            }
        };
        TradeQuery::new(market, &self.symbol, anchor, before, after)
    }

    pub fn add_fifty(&mut self, field: CountField) -> Result<(), QueryError> {
        let target = match field {
            CountField::Before => &mut self.before,
            CountField::After => &mut self.after,
        };
        let current = parse_count("数量", target)?;
        let next = current
            .checked_add(50)
            .ok_or_else(|| invalid("数量加 50 后超出范围"))?;
        *target = next.to_string();
        Ok(())
    }
}

fn parse_count(label: &str, input: &str) -> Result<u16, QueryError> {
    if input.is_empty() || !input.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(invalid(format!("{label}必须是非负整数")));
    }
    input
        .parse()
        .map_err(|_| invalid(format!("{label}超出 0..=65535 范围")))
}

fn invalid(message: impl Into<String>) -> QueryError {
    QueryError::new(ErrorCode::InvalidInput, message)
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TimeConversion {
    pub input: String,
    pub timestamp: Timestamp,
    pub utc: String,
    pub beijing: String,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct TimeHistory {
    entries: Vec<TimeConversion>,
}

impl TimeHistory {
    pub fn entries(&self) -> &[TimeConversion] {
        &self.entries
    }

    pub fn convert_and_push(
        &mut self,
        input: &str,
        unit: TimeUnitChoice,
    ) -> Result<TimeConversion, QueryError> {
        let timestamp = parse_time(input.trim(), unit.explicit())?;
        let conversion = TimeConversion {
            input: input.trim().into(),
            timestamp,
            utc: format_utc(timestamp)?,
            beijing: format_beijing(timestamp)?,
        };
        self.entries.insert(0, conversion.clone());
        Ok(conversion)
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum QueryPhase {
    #[default]
    Idle,
    Loading,
    Success,
    Empty,
    Partial,
    Error,
}

#[derive(Clone, Debug, Default)]
pub struct QueryState {
    generation: u64,
    phase: QueryPhase,
    window: Option<TradeWindow>,
    error: Option<String>,
}

impl QueryState {
    pub fn generation(&self) -> u64 {
        self.generation
    }

    pub fn phase(&self) -> QueryPhase {
        self.phase
    }

    pub fn window(&self) -> Option<&TradeWindow> {
        self.window.as_ref()
    }

    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    pub fn begin(&mut self) -> u64 {
        self.generation = self.generation.wrapping_add(1);
        self.phase = QueryPhase::Loading;
        self.error = None;
        self.generation
    }

    pub fn succeed(&mut self, generation: u64, window: TradeWindow) -> bool {
        if generation != self.generation {
            return false;
        }
        self.phase = if window.meta.partial {
            QueryPhase::Partial
        } else if window.items.is_empty() {
            QueryPhase::Empty
        } else {
            QueryPhase::Success
        };
        self.window = Some(window);
        self.error = None;
        true
    }

    pub fn fail(&mut self, generation: u64, error: String) -> bool {
        if generation != self.generation {
            return false;
        }
        self.phase = QueryPhase::Error;
        self.error = Some(error);
        true
    }
}

#[cfg(test)]
mod tests {
    use exchange_types::{Market, TimeUnit, TradeAnchor};
    use pretty_assertions::assert_eq;

    use super::{
        AnchorMode, CountField, QueryForm, QueryPhase, QueryState, TimeHistory, TimeUnitChoice,
    };

    #[test]
    fn parses_trade_id_as_full_u64_without_javascript_number_rounding() {
        let form = QueryForm {
            anchor_mode: AnchorMode::Id,
            anchor: "18446744073709551615".into(),
            ..QueryForm::default()
        };

        let query = form.parse(Market::Spot).expect("maximum u64 is valid");

        assert_eq!(query.anchor, TradeAnchor::Id(u64::MAX));
    }

    #[test]
    fn rejects_non_digit_trade_ids() {
        let form = QueryForm {
            anchor_mode: AnchorMode::Id,
            anchor: "9007199254740992.0".into(),
            ..QueryForm::default()
        };

        let error = form.parse(Market::Spot).expect_err("decimal ID must fail");

        assert_eq!(error.message, "成交 ID 只能包含数字");
    }

    #[test]
    fn parses_numeric_time_with_an_explicit_unit() {
        let form = QueryForm {
            anchor_mode: AnchorMode::Time,
            anchor: "1725897600123456".into(),
            time_unit: TimeUnitChoice::Micros,
            ..QueryForm::default()
        };

        let query = form.parse(Market::Usdm).expect("microseconds are valid");

        assert_eq!(
            query.anchor,
            TradeAnchor::Time(exchange_types::Timestamp {
                value: 1_725_897_600_123_456,
                unit: TimeUnit::Micros,
            })
        );
    }

    #[test]
    fn shared_limit_validation_rejects_more_than_one_thousand_rows() {
        let form = QueryForm {
            anchor: "42".into(),
            before: "500".into(),
            after: "500".into(),
            ..QueryForm::default()
        };

        let error = form.parse(Market::Spot).expect_err("1001 rows must fail");

        assert!(error.message.contains("1000"));
    }

    #[test]
    fn plus_fifty_updates_only_the_selected_count() {
        let mut form = QueryForm::default();

        form.add_fifty(CountField::After)
            .expect("default after count can grow");

        assert_eq!(form.before, "50");
        assert_eq!(form.after, "100");
    }

    #[test]
    fn plus_fifty_rejects_invalid_or_overflowing_counts() {
        let mut invalid = QueryForm {
            before: "-1".into(),
            ..QueryForm::default()
        };
        let mut overflow = QueryForm {
            after: u16::MAX.to_string(),
            ..QueryForm::default()
        };

        assert!(invalid.add_fifty(CountField::Before).is_err());
        assert!(overflow.add_fifty(CountField::After).is_err());
    }

    #[test]
    fn time_history_is_newest_first_and_invalid_input_does_not_change_it() {
        let mut history = TimeHistory::default();
        let first = history
            .convert_and_push("1725897600", TimeUnitChoice::Seconds)
            .expect("seconds are valid");
        let second = history
            .convert_and_push("1725897600123", TimeUnitChoice::Millis)
            .expect("milliseconds are valid");
        let snapshot = history.entries().to_vec();

        assert!(
            history
                .convert_and_push("bad time", TimeUnitChoice::Auto)
                .is_err()
        );
        assert_eq!(history.entries(), snapshot);
        assert_eq!(history.entries(), &[second, first]);
    }

    #[test]
    fn stale_generation_cannot_overwrite_a_newer_request() {
        let mut state = QueryState::default();
        let older = state.begin();
        let newer = state.begin();

        assert!(!state.fail(older, "旧请求失败".into()));
        assert_eq!(state.generation(), newer);
        assert!(matches!(state.phase(), QueryPhase::Loading));
    }
}

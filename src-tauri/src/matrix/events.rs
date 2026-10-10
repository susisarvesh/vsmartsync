//! Documented HTTP event fields from `geteventcount` and `events?action=getevent`.
//!
//! Unknown `event-id` values stay numeric text. This module does not interpret
//! them as access granted or denied, and it does not parse `RPL_EVT` text.

/// Events requested in one HTTP batch. Valid for Direct Door V2 and the other
/// doors listed in the guide table.
pub const EVENT_BATCH_SIZE: i64 = 5;

/// Current sequence and rollover from `command?action=geteventcount`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EventCount {
    pub roll_over_count: i64,
    pub seq_number: i64,
}

/// One event from a documented `getevent` body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedDeviceEvent {
    pub roll_over_count: i64,
    pub seq_number: i64,
    pub event_id: Option<String>,
    pub device_date: String,
    pub device_time: String,
    /// Named `ref-user-id`, `user-id`, or `reference-id` when present.
    /// Otherwise field 1 on a documented user event.
    pub ref_user_id: Option<i64>,
    pub detail_1: Option<String>,
    pub detail_2: Option<String>,
    pub detail_3: Option<String>,
    pub detail_4: Option<String>,
    pub detail_5: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EventBody {
    Events(Vec<ParsedDeviceEvent>),
    /// `Response-Code=0` and no event records.
    Empty,
    /// Body is not the documented field list. The caller must not advance the cursor.
    Unreadable,
}

/// Tag names and size only. Values stay out of the log.
pub fn response_markers(body: &str) -> String {
    let lower = body.to_ascii_lowercase();
    let mut names = Vec::new();
    let mut search = 0;
    while let Some(rel) = lower[search..].find('<') {
        let start = search + rel + 1;
        let lead = lower.as_bytes().get(start).copied();
        if lead == Some(b'/') || lead == Some(b'!') || lead == Some(b'?') {
            search = start;
            continue;
        }
        let rest = &lower[start..];
        let end = rest
            .find(|ch: char| ch.is_whitespace() || ch == '>' || ch == '/')
            .unwrap_or(rest.len());
        let name = &rest[..end];
        if !name.is_empty()
            && name.len() <= 40
            && name
                .chars()
                .all(|ch| ch.is_ascii_alphanumeric() || ch == '-')
            && !names.iter().any(|existing: &String| existing == name)
        {
            names.push(name.to_string());
        }
        if names.len() == 12 {
            break;
        }
        search = start + end.max(1);
    }
    format!("bytes={} tags={}", body.len(), names.join(","))
}

pub fn parse_event_count(body: &str) -> Option<EventCount> {
    let roll_over_count = first_i64(body, "roll-over-count")?;
    let seq_number = first_i64(body, "seq-number").or_else(|| first_i64(body, "seq-no"))?;
    if !plausible_cursor(roll_over_count, seq_number) {
        return None;
    }
    Some(EventCount {
        roll_over_count,
        seq_number,
    })
}

pub fn parse_device_events(body: &str) -> EventBody {
    let seq_numbers = field_values(body, "seq-no");
    let seq_numbers = if seq_numbers.is_empty() {
        field_values(body, "seq-number")
    } else {
        seq_numbers
    };
    if seq_numbers.is_empty() {
        return if looks_like_undocumented_event(body) {
            EventBody::Unreadable
        } else {
            EventBody::Empty
        };
    }

    let rollovers = field_values(body, "roll-over-count");
    let dates = field_values(body, "date");
    let times = field_values(body, "time");
    let event_ids = field_values(body, "event-id");
    let refs = named_refs(body, seq_numbers.len());
    let detail_1 = field_values(body, "detail-1");
    let detail_2 = field_values(body, "detail-2");
    let detail_3 = field_values(body, "detail-3");
    let detail_4 = field_values(body, "detail-4");
    let detail_5 = field_values(body, "detail-5");

    let mut events = Vec::new();
    for (index, seq_raw) in seq_numbers.iter().enumerate() {
        let Some(seq_number) = parse_i64(seq_raw) else {
            return EventBody::Unreadable;
        };
        let Some(roll_over_count) = shared_or_aligned_i64(&rollovers, index) else {
            return EventBody::Unreadable;
        };
        if !plausible_cursor(roll_over_count, seq_number) {
            return EventBody::Unreadable;
        }
        events.push(ParsedDeviceEvent {
            roll_over_count,
            seq_number,
            event_id: aligned_text(&event_ids, index),
            device_date: aligned_text(&dates, index).unwrap_or_default(),
            device_time: aligned_text(&times, index).unwrap_or_default(),
            ref_user_id: refs.get(index).copied().flatten().or_else(|| {
                reference_in_field1(
                    aligned_text(&event_ids, index).as_deref(),
                    aligned_text(&detail_1, index).as_deref(),
                )
            }),
            detail_1: aligned_text(&detail_1, index),
            detail_2: aligned_text(&detail_2, index),
            detail_3: aligned_text(&detail_3, index),
            detail_4: aligned_text(&detail_4, index),
            detail_5: aligned_text(&detail_5, index),
        });
    }
    EventBody::Events(events)
}

fn looks_like_undocumented_event(body: &str) -> bool {
    let lower = body.to_ascii_lowercase();
    lower.contains("rpl_evt") || lower.contains("event-id") || lower.contains("detail-1")
}

fn plausible_cursor(roll_over_count: i64, seq_number: i64) -> bool {
    (0..=65_535).contains(&roll_over_count) && (0..=500_000).contains(&seq_number)
}

/// Guide user events store the reference user id in field 1 (`detail-1`).
/// Door and alarm events do not. A named `user-id` still wins when the body has one.
pub fn reference_in_field1(event_id: Option<&str>, detail_1: Option<&str>) -> Option<i64> {
    let event_id = event_id?.trim().parse::<u16>().ok()?;
    if !matches!(event_id, 101..=110 | 151..=166 | 405) {
        return None;
    }
    parse_ref(detail_1?)
}

pub fn user_event_ids_with_field1_user() -> Vec<String> {
    (101..=110)
        .chain(151..=166)
        .chain(std::iter::once(405))
        .map(|id| id.to_string())
        .collect()
}

fn named_refs(body: &str, count: usize) -> Vec<Option<i64>> {
    for key in ["ref-user-id", "user-id", "reference-id"] {
        let values = field_values(body, key);
        if values.len() == count {
            return values.iter().map(|value| parse_ref(value)).collect();
        }
    }
    vec![None; count]
}

fn parse_ref(value: &str) -> Option<i64> {
    let parsed = parse_i64(value)?;
    if (0..=99_999_999).contains(&parsed) {
        Some(parsed)
    } else {
        None
    }
}

/// One rollover on a batch applies to every event. A missing per-event value does not.
fn shared_or_aligned_i64(values: &[String], index: usize) -> Option<i64> {
    let raw = if values.len() == 1 {
        values.first()
    } else {
        values.get(index)
    }?;
    parse_i64(raw)
}

fn aligned_text(values: &[String], index: usize) -> Option<String> {
    let raw = values.get(index)?;
    let clipped = clip(raw);
    if clipped.is_empty() {
        None
    } else {
        Some(clipped)
    }
}

fn clip(value: &str) -> String {
    value.trim().chars().take(128).collect()
}

fn first_i64(body: &str, key: &str) -> Option<i64> {
    field_values(body, key)
        .first()
        .and_then(|value| parse_i64(value))
}

fn parse_i64(value: &str) -> Option<i64> {
    let trimmed = value.trim();
    if trimmed.is_empty() || !trimmed.chars().all(|ch| ch.is_ascii_digit()) {
        return None;
    }
    trimmed.parse().ok()
}

fn field_values(body: &str, key: &str) -> Vec<String> {
    let xml = xml_values(body, key);
    if !xml.is_empty() {
        return xml;
    }
    text_values(body, key)
}

fn xml_values(body: &str, key: &str) -> Vec<String> {
    let lower = body.to_ascii_lowercase();
    let key = key.to_ascii_lowercase();
    let start_tag = format!("<{key}>");
    let end_tag = format!("</{key}>");
    let mut values = Vec::new();
    let mut search = 0;
    while let Some(rel) = lower[search..].find(&start_tag) {
        let start = search + rel + start_tag.len();
        let Some(end_rel) = lower[start..].find(&end_tag) else {
            break;
        };
        values.push(body[start..start + end_rel].trim().to_string());
        search = start + end_rel + end_tag.len();
    }
    values
}

fn text_values(body: &str, key: &str) -> Vec<String> {
    let lower = body.to_ascii_lowercase();
    let needle = format!("{}=", key.to_ascii_lowercase());
    let mut values = Vec::new();
    let mut search = 0;
    while let Some(rel) = lower[search..].find(&needle) {
        let start = search + rel;
        let boundary = start == 0
            || lower.as_bytes()[start - 1].is_ascii_whitespace()
            || lower.as_bytes()[start - 1] == b'>';
        if boundary {
            let value_start = start + needle.len();
            let rest = &body[value_start..];
            let end = rest
                .find(|ch: char| ch.is_whitespace() || ch == '<')
                .unwrap_or(rest.len());
            values.push(rest[..end].trim().to_string());
            search = value_start + end;
        } else {
            search = start + needle.len();
        }
    }
    values
}

#[cfg(test)]
mod tests {
    use super::{parse_device_events, parse_event_count, EventBody};

    #[test]
    fn parses_documented_xml_batch_and_named_user_id() {
        let body = r#"
            <COSEC_API>
            <Response-Code>0</Response-Code>
            <roll-over-count>6</roll-over-count>
            <seq-No>1141</seq-No>
            <date>31122022</date>
            <time>103008</time>
            <event-id>101</event-id>
            <user-id>51579</user-id>
            <detail-1>0</detail-1>
            <detail-2>20</detail-2>
            <seq-No>1142</seq-No>
            <date>31122022</date>
            <time>103045</time>
            <event-id>102</event-id>
            <user-id>48218</user-id>
            <detail-1>1</detail-1>
            </COSEC_API>
        "#;
        let EventBody::Events(events) = parse_device_events(body) else {
            panic!("expected events");
        };
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].roll_over_count, 6);
        assert_eq!(events[0].seq_number, 1141);
        assert_eq!(events[0].event_id.as_deref(), Some("101"));
        assert_eq!(events[0].device_date, "31122022");
        assert_eq!(events[0].device_time, "103008");
        assert_eq!(events[0].ref_user_id, Some(51579));
        assert_eq!(events[0].detail_1.as_deref(), Some("0"));
        assert_eq!(events[0].detail_2.as_deref(), Some("20"));
        assert_eq!(events[1].seq_number, 1142);
        assert_eq!(events[1].ref_user_id, Some(48218));
        assert!(events[1].detail_2.is_none());
    }

    #[test]
    fn success_without_events_is_empty() {
        assert_eq!(
            parse_device_events("<COSEC_API><Response-Code>0</Response-Code></COSEC_API>"),
            EventBody::Empty
        );
    }

    #[test]
    fn rpl_evt_text_is_not_guessed() {
        let body = "[RPL_EVT&6&1140&  {   0&14   } &1&31122022&103008&101&51579&0&20&]";
        assert_eq!(parse_device_events(body), EventBody::Unreadable);
    }

    #[test]
    fn user_allowed_event_reads_the_reference_from_field1() {
        let body = r#"
            <COSEC_API>
            <Response-Code>0</Response-Code>
            <roll-over-count>1</roll-over-count>
            <seq-No>20</seq-No>
            <date>10102026</date>
            <time>143526</time>
            <event-id>101</event-id>
            <detail-1>10000001</detail-1>
            <detail-2>0</detail-2>
            <detail-3>640</detail-3>
            <detail-4>0</detail-4>
            </COSEC_API>
        "#;
        let EventBody::Events(events) = parse_device_events(body) else {
            panic!("expected events");
        };
        assert_eq!(events[0].ref_user_id, Some(10_000_001));
        assert_eq!(events[0].detail_1.as_deref(), Some("10000001"));
    }

    #[test]
    fn door_event_does_not_treat_field1_as_a_user() {
        let body = r#"
            <COSEC_API>
            <Response-Code>0</Response-Code>
            <roll-over-count>1</roll-over-count>
            <seq-No>21</seq-No>
            <event-id>201</event-id>
            <detail-1>10000001</detail-1>
            </COSEC_API>
        "#;
        let EventBody::Events(events) = parse_device_events(body) else {
            panic!("expected events");
        };
        assert_eq!(events[0].ref_user_id, None);
    }

    #[test]
    fn event_count_reads_seq_number() {
        let count = parse_event_count(
            "<COSEC_API><roll-over-count>6</roll-over-count><seq-number>1140</seq-number></COSEC_API>",
        )
        .unwrap();
        assert_eq!(count.roll_over_count, 6);
        assert_eq!(count.seq_number, 1140);
    }
}

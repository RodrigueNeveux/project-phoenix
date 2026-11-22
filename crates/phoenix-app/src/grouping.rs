//! Message grouping: consecutive messages by the same author within
//! 7 minutes collapse under one header (avatar + name), like Discord.

use crate::model::Message;

pub const GROUP_WINDOW_SECS: i64 = 7 * 60;

/// Does `cur` start a new group (render avatar + username header)?
pub fn starts_group(prev: Option<&Message>, cur: &Message, utc_offset_secs: i64) -> bool {
    let Some(prev) = prev else { return true };
    if prev.author != cur.author || cur.reply_to.is_some() {
        return true;
    }
    let dt = cur.timestamp - prev.timestamp;
    if !(0..=GROUP_WINDOW_SECS).contains(&dt) {
        return true;
    }
    day(prev.timestamp, utc_offset_secs) != day(cur.timestamp, utc_offset_secs)
}

/// Should a day divider be drawn above `cur`?
pub fn day_divider(prev: Option<&Message>, cur: &Message, utc_offset_secs: i64) -> bool {
    prev.is_some_and(|p| day(p.timestamp, utc_offset_secs) != day(cur.timestamp, utc_offset_secs))
}

fn day(ts: i64, off: i64) -> i64 {
    (ts + off).div_euclid(86_400)
}

/// Compute group-start flags for a whole slice.
pub fn group_flags(msgs: &[Message], utc_offset_secs: i64) -> Vec<bool> {
    msgs.iter()
        .enumerate()
        .map(|(i, m)| starts_group(i.checked_sub(1).map(|p| &msgs[p]), m, utc_offset_secs))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Id;

    fn msg(id: u64, author: u64, ts: i64) -> Message {
        Message {
            id: Id(id),
            channel: Id(1),
            author: Id(author),
            timestamp: ts,
            edited: false,
            content: "x".into(),
            reply_to: None,
            embeds: vec![],
            attachments: vec![],
            reactions: vec![],
        }
    }

    const T0: i64 = 1_763_800_000; // a weekday mid-day UTC

    #[test]
    fn first_message_starts_group() {
        assert!(starts_group(None, &msg(1, 1, T0), 0));
    }

    #[test]
    fn same_author_within_window_groups() {
        let a = msg(1, 1, T0);
        let b = msg(2, 1, T0 + 60);
        assert!(!starts_group(Some(&a), &b, 0));
        let c = msg(3, 1, T0 + GROUP_WINDOW_SECS);
        assert!(!starts_group(Some(&a), &c, 0));
    }

    #[test]
    fn window_expires_after_seven_minutes() {
        let a = msg(1, 1, T0);
        let b = msg(2, 1, T0 + GROUP_WINDOW_SECS + 1);
        assert!(starts_group(Some(&a), &b, 0));
    }

    #[test]
    fn different_author_breaks_group() {
        assert!(starts_group(Some(&msg(1, 1, T0)), &msg(2, 2, T0 + 5), 0));
    }

    #[test]
    fn reply_always_starts_group() {
        let mut b = msg(2, 1, T0 + 5);
        b.reply_to = Some(Id(1));
        assert!(starts_group(Some(&msg(1, 1, T0)), &b, 0));
    }

    #[test]
    fn day_boundary_breaks_group() {
        let midnight = (T0 / 86_400 + 1) * 86_400;
        let a = msg(1, 1, midnight - 60);
        let b = msg(2, 1, midnight + 60);
        assert!(starts_group(Some(&a), &b, 0));
        assert!(day_divider(Some(&a), &b, 0));
        // With a +2h offset both fall on the same local day.
        assert!(!starts_group(Some(&a), &b, 7200));
    }

    #[test]
    fn out_of_order_timestamps_break_group() {
        assert!(starts_group(Some(&msg(1, 1, T0)), &msg(2, 1, T0 - 10), 0));
    }

    #[test]
    fn flags_for_slice() {
        let v = vec![
            msg(1, 1, T0),
            msg(2, 1, T0 + 10),
            msg(3, 2, T0 + 20),
            msg(4, 2, T0 + 30),
            msg(5, 2, T0 + 1000),
        ];
        assert_eq!(group_flags(&v, 0), vec![true, false, true, false, true]);
    }
}

//! The clock text: a format with `%` tokens that may span several lines, each line with its own
//! alignment. In the format a new line is `\n` and a line may start with `<` (left), `^`
//! (centered, the default) or `>` (right).
//!
//! Tokens: `%H` hour 00-23, `%I` hour 01-12, `%M` minute, `%S` second, `%p` AM or PM, `%d` day of
//! the month 01-31, `%e` the same padded with a space, `%m` month 01-12, `%y` year 00-99, `%Y` year,
//! `%P` the same in lower case, `%a` and `%A` short and full weekday, `%b` (or `%h`) and `%B`
//! short and full month, `%%` a percent sign.
//! The names are English. An unknown token is shown as it was typed.

pub const DEFAULT_FORMAT: &str = "%H:%M";

const WEEKDAYS: [&str; 7] = [
    "Sunday",
    "Monday",
    "Tuesday",
    "Wednesday",
    "Thursday",
    "Friday",
    "Saturday",
];
const MONTHS: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Time {
    pub year: u16,
    /// 1 to 12.
    pub month: u16,
    /// 0 is Sunday.
    pub weekday: u16,
    pub day: u16,
    pub hour: u16,
    pub minute: u16,
    pub second: u16,
}

impl Time {
    pub fn hm(hour: u16, minute: u16) -> Time {
        Time {
            hour,
            minute,
            ..Time::default()
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Align {
    Left,
    #[default]
    Center,
    Right,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Line {
    pub text: String,
    pub align: Align,
}

fn normalize(format: &str) -> String {
    if format.contains('%') {
        return format.to_string();
    }
    let mut s = format.to_string();
    s = s.replace("DDDD", "%A").replace("dddd", "%A");
    s = s.replace("DDD", "%a").replace("ddd", "%a");
    s = s.replace("MMMM", "%B");
    s = s.replace("MMM", "%b");
    let has_ampm =
        s.contains("tt") || s.contains("TT") || s.contains("am/pm") || s.contains("AM/PM");
    s = s
        .replace("tt", "%p")
        .replace("TT", "%p")
        .replace("am/pm", "%P")
        .replace("AM/PM", "%p");
    if has_ampm {
        s = s.replace("hh", "%I");
    } else {
        s = s.replace("hh", "%H");
    }
    s = s.replace("HH", "%H");
    s = s.replace("mm", "%M");
    s = s.replace("ss", "%S");
    s = s.replace("yyyy", "%Y").replace("YYYY", "%Y");
    s = s.replace("yy", "%y").replace("YY", "%y");
    s = s.replace("dd", "%d");
    s = s.replace("MM", "%m");
    s
}

fn lines_of(format: &str) -> Vec<(Align, String)> {
    let norm = normalize(format);
    norm.replace("\\n", "\n")
        .split('\n')
        .map(|line| match line.chars().next() {
            Some('<') => (Align::Left, line[1..].to_string()),
            Some('^') => (Align::Center, line[1..].to_string()),
            Some('>') => (Align::Right, line[1..].to_string()),
            _ => (Align::Center, line.to_string()),
        })
        .collect()
}

fn name(names: &[&str], index: usize, short: bool) -> String {
    let full = names.get(index).copied().unwrap_or("?");
    if short {
        full.chars().take(3).collect()
    } else {
        full.to_string()
    }
}

fn expand(text: &str, time: &Time) -> String {
    let mut out = String::new();
    let mut chars = text.chars();
    while let Some(c) = chars.next() {
        if c != '%' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('H') => out.push_str(&format!("{:02}", time.hour)),
            Some('I') => out.push_str(&format!("{:02}", (time.hour + 11) % 12 + 1)),
            Some('M') => out.push_str(&format!("{:02}", time.minute)),
            Some('S') => out.push_str(&format!("{:02}", time.second)),
            Some('p') => out.push_str(if time.hour < 12 { "AM" } else { "PM" }),
            Some('P') => out.push_str(if time.hour < 12 { "am" } else { "pm" }),
            Some('d') => out.push_str(&format!("{:02}", time.day)),
            Some('e') => out.push_str(&format!("{:2}", time.day)),
            Some('m') => out.push_str(&format!("{:02}", time.month)),
            Some('y') => out.push_str(&format!("{:02}", time.year % 100)),
            Some('Y') => out.push_str(&time.year.to_string()),
            Some('a') => out.push_str(&name(&WEEKDAYS, usize::from(time.weekday), true)),
            Some('A') => out.push_str(&name(&WEEKDAYS, usize::from(time.weekday), false)),
            Some('b' | 'h') => out.push_str(&name(
                &MONTHS,
                usize::from(time.month).saturating_sub(1),
                true,
            )),
            Some('B') => out.push_str(&name(
                &MONTHS,
                usize::from(time.month).saturating_sub(1),
                false,
            )),
            Some('%') => out.push('%'),
            Some(other) => {
                out.push('%');
                out.push(other);
            }
            None => out.push('%'),
        }
    }
    out
}

/// The lines of the clock for `time`.
pub fn render(format: &str, time: &Time) -> Vec<Line> {
    lines_of(format)
        .into_iter()
        .map(|(align, text)| Line {
            text: expand(&text, time),
            align,
        })
        .collect()
}

/// The most characters a token can produce, so the clock keeps its size as the text changes.
fn widest(token: char) -> usize {
    match token {
        'A' => WEEKDAYS.iter().map(|n| n.len()).max().unwrap_or(9),
        'B' => MONTHS.iter().map(|n| n.len()).max().unwrap_or(9),
        'a' | 'b' | 'h' => 3,
        'Y' => 4,
        '%' => 1,
        _ => 2,
    }
}

fn columns(text: &str) -> usize {
    let mut count = 0;
    let mut chars = text.chars();
    while let Some(c) = chars.next() {
        if c == '%' {
            count += chars.next().map_or(1, widest);
        } else {
            count += 1;
        }
    }
    count
}

/// How many lines and the most columns the format can produce.
pub fn size_of(format: &str) -> (usize, usize) {
    let lines = lines_of(format);
    let wide = lines
        .iter()
        .map(|(_, text)| columns(text))
        .max()
        .unwrap_or(0);
    (lines.len().max(1), wide)
}

/// Pixels at 100% scale the clock takes along the bar: wide enough for the longest line on a top
/// or bottom bar, tall enough for all the lines on a side bar. One short line gives the old size.
pub fn extent(format: &str, horizontal: bool) -> f32 {
    let (rows, wide) = size_of(format);
    if horizontal {
        (wide as f32 * 8.0 + 16.0).max(80.0)
    } else {
        (rows as f32 * 17.0 + 13.0).max(30.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at() -> Time {
        Time {
            year: 2026,
            month: 11,
            weekday: 1,
            day: 3,
            hour: 15,
            minute: 7,
            second: 9,
        }
    }

    fn texts(format: &str) -> Vec<String> {
        render(format, &at()).into_iter().map(|l| l.text).collect()
    }

    #[test]
    fn the_default_is_hours_and_minutes_zero_padded() {
        assert_eq!(texts(DEFAULT_FORMAT), ["15:07"]);
        assert_eq!(render(DEFAULT_FORMAT, &Time::hm(7, 5))[0].text, "07:05");
    }

    #[test]
    fn tokens_expand() {
        assert_eq!(texts("%I:%M:%S %p"), ["03:07:09 PM"]);
        assert_eq!(texts("%a %e %b %Y"), ["Mon  3 Nov 2026"]);
        assert_eq!(texts("%A, %B %d, %y"), ["Monday, November 03, 26"]);
        assert_eq!(texts("%m/%d 100%%"), ["11/03 100%"]);
        assert_eq!(texts("%I %p"), ["03 PM"]);
        assert_eq!(texts("%I"), ["03"]);
    }

    #[test]
    fn midnight_and_noon_read_twelve() {
        assert_eq!(render("%I", &Time::hm(0, 0))[0].text, "12");
        assert_eq!(render("%I", &Time::hm(12, 0))[0].text, "12");
    }

    #[test]
    fn unknown_tokens_stay_as_typed() {
        assert_eq!(texts("%Q 5%"), ["%Q 5%"]);
    }

    #[test]
    fn lines_and_alignment() {
        let lines = render("<%H:%M\\n^%d\n>%b", &at());
        assert_eq!(
            lines,
            vec![
                Line {
                    text: "15:07".into(),
                    align: Align::Left
                },
                Line {
                    text: "03".into(),
                    align: Align::Center
                },
                Line {
                    text: "Nov".into(),
                    align: Align::Right
                },
            ]
        );
    }

    #[test]
    fn size_does_not_depend_on_the_time() {
        assert_eq!(size_of("%A"), (1, 9));
        assert_eq!(size_of("%H:%M\n%a %d %b"), (2, 10));
        assert_eq!(extent(DEFAULT_FORMAT, true), 80.0);
        assert_eq!(extent(DEFAULT_FORMAT, false), 30.0);
        assert!(extent("%A %d %B %Y", true) > 80.0);
        assert!(extent("%H:%M\n%a\n%d", false) > 30.0);
    }

    #[test]
    fn windows_style_format_tokens() {
        let t = Time {
            year: 2026,
            month: 10,
            weekday: 4,
            day: 26,
            hour: 15,
            minute: 7,
            second: 9,
        };
        let lines = render("hh:mm:ss\nDDD, dd MMM", &t);
        assert_eq!(lines[0].text, "15:07:09");
        assert_eq!(lines[1].text, "Thu, 26 Oct");
    }
}

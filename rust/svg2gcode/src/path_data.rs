//! Port of svg/ParseSvgPathData.go: tokenizes an SVG path `d` attribute into
//! a flat list of commands, each carrying every number that follows it up
//! to the next command letter. Splitting those numbers into per-call
//! argument groups (and per-arg-group implicit-lineto repetition) is a
//! per-command concern handled by the path handlers, not the tokenizer --
//! this mirrors Go exactly (see e.g. MoveAbsolute recursing into
//! LineToAbsolute for the trailing coordinate pairs).

use crate::number::pop_number;

#[derive(Debug, Clone, PartialEq)]
pub struct PathCommand {
    pub command: char,
    pub parameters: Vec<f64>,
}

const COMMAND_LETTERS: &[char] = &[
    'M', 'm', 'L', 'l', 'H', 'h', 'V', 'v', 'C', 'c', 'S', 's', 'Q', 'q', 'T', 't', 'A', 'a', 'Z',
    'z',
];

fn trim_space_and_comma(input: &str) -> &str {
    let mut result = input;
    loop {
        let next = result.trim();
        let next = next.strip_prefix(',').unwrap_or(next);
        let next = next.trim();
        if next == result {
            return result;
        }
        result = next;
    }
}

fn pop_svg_path_parameters(input: &str) -> (Vec<f64>, &str) {
    let mut result = Vec::new();
    let mut remaining = input;
    loop {
        remaining = trim_space_and_comma(remaining);
        match pop_number(remaining) {
            Ok((value, rest)) => {
                result.push(value);
                remaining = rest;
            }
            Err(_) => return (result, remaining),
        }
    }
}

pub fn parse_svg_path_data(input: &str) -> Vec<PathCommand> {
    let mut result = Vec::new();
    let mut remaining = input;
    loop {
        remaining = remaining.trim();
        if remaining.is_empty() {
            return result;
        }
        let command = remaining.chars().next().unwrap();
        remaining = &remaining[command.len_utf8()..];
        if COMMAND_LETTERS.contains(&command) {
            let (parameters, rest) = pop_svg_path_parameters(remaining);
            remaining = rest;
            result.push(PathCommand {
                command,
                parameters,
            });
        }
        // else: unexpected command character; Go logs a warning and moves
        // on (it already advanced past the character above).
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_svg_path_data() {
        assert_eq!(
            parse_svg_path_data("M 0,1 A 2,3"),
            vec![
                PathCommand {
                    command: 'M',
                    parameters: vec![0.0, 1.0]
                },
                PathCommand {
                    command: 'A',
                    parameters: vec![2.0, 3.0]
                },
            ]
        );
        assert_eq!(parse_svg_path_data("+"), vec![]);
        assert_eq!(
            parse_svg_path_data("M1,2"),
            vec![PathCommand {
                command: 'M',
                parameters: vec![1.0, 2.0]
            }]
        );
        assert_eq!(
            parse_svg_path_data("M 0,0 25.4,25.4"),
            vec![PathCommand {
                command: 'M',
                parameters: vec![0.0, 0.0, 25.4, 25.4]
            }]
        );
    }
}

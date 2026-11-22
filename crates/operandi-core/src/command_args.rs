//! Command-line argument conventions (spec `command-conventions-2026-11-05`):
//! arguments by NAME:VALUE in any order or by position, choices by name or
//! index, and booleans as true/false, on/off or 1/0.
use crate::Result;

/// Binds `args` (without the command name) to the declared `names`.
/// NAME:VALUE pairs go to their name (names are case-insensitive); bare
/// values fill the remaining names in declared order. Unknown names, a name
/// given twice and too many values are refused (provisional, Q-CV-04).
pub fn bind<'a>(command: &str, args: &'a [String], names: &[&str]) -> Result<Vec<Option<&'a str>>> {
    let mut bound: Vec<Option<&str>> = vec![None; names.len()];
    let mut positional = Vec::new();
    for arg in args {
        let named = arg.split_once(':').and_then(|(key, value)| {
            names
                .iter()
                .position(|n| n.eq_ignore_ascii_case(key))
                .map(|i| (i, value))
        });
        match named {
            Some((i, value)) => {
                if bound[i].is_some() {
                    return Err(format!("{command}: {} given twice", names[i]));
                }
                bound[i] = Some(value);
            }
            None => {
                if let Some((key, _)) = arg.split_once(':')
                    && !key.is_empty()
                    && key.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
                    && !key.chars().all(|c| c.is_ascii_digit())
                {
                    return Err(format!(
                        "{command}: unknown argument {key}; arguments are {}",
                        names.join(", ")
                    ));
                }
                positional.push(arg.as_str());
            }
        }
    }
    let mut free = (0..names.len())
        .filter(|&i| bound[i].is_none())
        .collect::<Vec<_>>()
        .into_iter();
    for value in positional {
        let i = free.next().ok_or_else(|| {
            format!(
                "{command}: too many arguments; arguments are {}",
                names.join(", ")
            )
        })?;
        bound[i] = Some(value);
    }
    Ok(bound)
}

/// A choice by name (case-insensitive) or by index. Unknown values are
/// refused with the valid choices (decided Q-CV-01).
pub fn choice(argument: &str, value: &str, choices: &[&str]) -> Result<usize> {
    if let Some(i) = choices.iter().position(|c| c.eq_ignore_ascii_case(value)) {
        return Ok(i);
    }
    if let Ok(i) = value.parse::<usize>()
        && i < choices.len()
    {
        return Ok(i);
    }
    Err(format!(
        "Unknown {argument} choice \"{value}\"; valid choices are {}",
        choices.join(", ")
    ))
}

/// A boolean as true/false, on/off, yes/no or 1/0.
pub fn boolean(argument: &str, value: &str) -> Result<bool> {
    match value.to_ascii_lowercase().as_str() {
        "true" | "on" | "yes" | "1" => Ok(true),
        "false" | "off" | "no" | "0" => Ok(false),
        _ => Err(format!(
            "{argument} must be true or false (on/off, 1/0), not \"{value}\""
        )),
    }
}

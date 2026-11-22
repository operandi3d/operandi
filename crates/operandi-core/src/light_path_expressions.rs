//! Light Path Expressions Render Output (render outputs group D,
//! 2026-11-20, RO-Q22..RO-Q27).
//!
//! An expression keeps the journeys of light from the emitters to the
//! camera that match a pattern (`C<RD>L`: light that bounced once off a
//! diffuse surface). The renderer has no expression engine, only fixed
//! light passes and light groups, so [`recipe`] parses the expression,
//! normalises it and maps it onto those: a list of [`Term`]s to add up and
//! a [`Status`] saying whether that is the exact answer, an approximation
//! or not possible (D-Q3: unsupported expressions render black with a
//! note). The renderer side turns terms into passes
//! (`operandi_cycles::outputs`).
//!
//! The language (a subset of the usual event letters): `C` first; scatter
//! events `<RD>` diffuse reflection, `<RS>` glossy reflection, `<TD>`
//! diffuse transmission, `<TS>` specular transmission, short forms `R`,
//! `T`, `D`, `S`; end events `L` light item, `O` glowing surface, `B`
//! background / environment, `V` volume; `.` any one event; `*` only right
//! after `.` and once; `[ ]` either-of; `'label'` right after an event.

/// The scatter and end events, as bits.
const RD: u8 = 1;
const RS: u8 = 2;
const TD: u8 = 4;
const TS: u8 = 8;
const L: u8 = 16;
const O: u8 = 32;
const B: u8 = 64;
const V: u8 = 128;
const SCATTER: u8 = RD | RS | TD | TS;
const END: u8 = L | O | B | V;
const ANY: u8 = SCATTER | END;
/// The emitters a direct or indirect light pass counts.
const EMITTERS: u8 = L | O | B;

/// The renderer's light pass families (its diffuse holds both diffuse
/// reflection and diffuse transmission).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Scatter {
    Diffuse,
    Glossy,
    Transmission,
}

/// Which bounces of a light pass: the first only, the later ones, or all.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Part {
    Direct,
    Indirect,
    Both,
}

/// A light group an expression reads.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Group {
    /// Every light item.
    AllLights,
    /// Lights (and the environment) with this LPE Label.
    Label(String),
    /// Every glowing surface.
    Emitters,
    /// The environment.
    World,
}

/// One piece of the picture; an expression's picture is their sum.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Term {
    /// The whole picture (before image processing).
    Combined,
    /// The background seen directly.
    Background,
    /// Glowing surfaces (and lamps) seen directly.
    Emission,
    /// Light scattered in volumes.
    Volume(Part),
    /// A family's light times its colour.
    Shading(Scatter, Part),
    /// A light group's whole picture.
    Group(Group),
}

/// How well the renderer can give an expression (the status line under
/// the field, never a modal error).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Status {
    /// Nothing typed yet.
    Empty,
    Exact,
    Approximate(String),
    /// Renders black.
    Unsupported(String),
    /// A bad expression: renders black. `at` is the 1-based character.
    Invalid {
        at: usize,
        message: String,
    },
}

impl Status {
    /// The one-line status shown under the field and in the render window.
    pub fn line(&self) -> String {
        match self {
            Status::Empty => "Type an expression".into(),
            Status::Exact => "Exact".into(),
            Status::Approximate(why) => format!("Approximate: {why}"),
            Status::Unsupported(why) => format!("Not supported by the renderer: {why}"),
            Status::Invalid { at, message } => format!("{message} (at character {at})"),
        }
    }
    /// Whether the output gets a picture (otherwise it renders black).
    pub fn renders(&self) -> bool {
        matches!(self, Status::Exact | Status::Approximate(_))
    }
}

/// What an expression renders as.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Recipe {
    /// Added together; empty when it renders black.
    pub terms: Vec<Term>,
    pub status: Status,
    /// For an unsupported expression: the nearest exact preset, for the
    /// tooltip.
    pub fallback: Option<String>,
}

/// The presets beside the field (our own names), in menu order.
pub const PRESETS: &[(&str, &str)] = &[
    ("All Light", "C.*"),
    ("Background Seen Directly", "CB"),
    ("Glowing Surfaces Seen Directly", "CO"),
    ("Volumes Seen Directly", "CV"),
    ("Direct Diffuse", "C<RD>L"),
    ("All Diffuse", "C<RD>.*"),
    ("Direct Reflections (highlights)", "C<RS>L"),
    ("All Reflections", "C<RS>.*"),
    ("Caustics", "CDS.*"),
];

/// The scene's own presets: "All Lights" and one "Light: <label>" per LPE
/// Label (labels that hold a quote can't be written in an expression and
/// are left out).
pub fn label_presets(labels: &[String]) -> Vec<(String, String)> {
    if labels.is_empty() {
        return Vec::new();
    }
    std::iter::once(("All Lights".to_owned(), "C.*L".to_owned()))
        .chain(
            labels
                .iter()
                .filter(|l| !l.is_empty() && !l.contains('\''))
                .map(|l| (format!("Light: {l}"), per_light_expression(l))),
        )
        .collect()
}

/// The expression for one light group: `C.*L'<label>'`.
pub fn per_light_expression(label: &str) -> String {
    format!("C.*L'{label}'")
}

/// One event of a parsed expression: the events it allows and its label.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Step {
    events: u8,
    label: Option<String>,
    /// `.*`: any number (zero or more) of any event.
    star: bool,
    /// Where it starts (1-based character), for messages.
    at: usize,
    /// A `[ ]` group (takes no label).
    grouped: bool,
}

fn invalid(at: usize, message: impl Into<String>) -> Status {
    Status::Invalid {
        at,
        message: message.into(),
    }
}

/// One scatter letter's events.
fn short_scatter(c: char) -> Option<u8> {
    Some(match c {
        'R' => RD | RS,
        'T' => TD | TS,
        'D' => RD | TD,
        'S' => RS | TS,
        _ => return None,
    })
}

fn end_letter(c: char) -> Option<u8> {
    Some(match c {
        'L' => L,
        'O' => O,
        'B' => B,
        'V' => V,
        _ => return None,
    })
}

/// `<XY>` (or `<XY'label'>`) after its `<`: X = R, T or `.`, Y = D, S or
/// `.`. Returns its events and label.
fn angle_event(chars: &[char], i: &mut usize) -> Result<(u8, Option<String>), Status> {
    let start = *i;
    let expected = || {
        invalid(
            start + 1,
            "Expected <RD>, <RS>, <TD> or <TS> (or . for either letter)",
        )
    };
    let x = chars.get(*i + 1).copied();
    let y = chars.get(*i + 2).copied();
    let mut close = *i + 3;
    let mut label = None;
    if chars.get(close) == Some(&'\'') {
        let Some(len) = chars[close + 1..].iter().position(|&c| c == '\'') else {
            return Err(invalid(close + 1, "This label has no closing '"));
        };
        label = Some(chars[close + 1..close + 1 + len].iter().collect());
        close += len + 2;
    }
    if chars.get(close) != Some(&'>') {
        return Err(expected());
    }
    let refl = match x {
        Some('R') => RD | RS,
        Some('T') => TD | TS,
        Some('.') => SCATTER,
        _ => return Err(expected()),
    };
    let kind = match y {
        Some('D') => RD | TD,
        Some('S') => RS | TS,
        Some('.') => SCATTER,
        _ => return Err(expected()),
    };
    *i = close + 1;
    Ok((refl & kind, label))
}

/// Reads one event (a letter, `<XY>` or `.`) at `i`, with the label
/// written inside `< >`.
fn event(chars: &[char], i: &mut usize) -> Result<Option<(u8, Option<String>)>, Status> {
    let c = chars[*i];
    if c == '<' {
        return angle_event(chars, i).map(Some);
    }
    let bits = match c {
        '.' => ANY,
        _ => match short_scatter(c).or_else(|| end_letter(c)) {
            Some(b) => b,
            None => return Ok(None),
        },
    };
    *i += 1;
    Ok(Some((bits, None)))
}

/// Parses `text` into its events after `C`.
fn parse(text: &str) -> Result<Vec<Step>, Status> {
    let chars: Vec<char> = text.chars().filter(|c| !c.is_whitespace()).collect();
    if chars.is_empty() {
        return Err(Status::Empty);
    }
    if chars[0] != 'C' {
        return Err(invalid(1, "An expression starts with C (the camera)"));
    }
    let mut steps: Vec<Step> = Vec::new();
    let mut i = 1;
    while i < chars.len() {
        let at = i + 1;
        match chars[i] {
            '*' => {
                let Some(last) = steps.last_mut() else {
                    return Err(invalid(at, "* only follows ."));
                };
                if last.events != ANY || last.grouped || last.label.is_some() || last.star {
                    return Err(invalid(at, "* only follows ."));
                }
                if steps.iter().any(|s| s.star) {
                    return Err(invalid(at, "Only one .* in an expression"));
                }
                if let Some(last) = steps.last_mut() {
                    last.star = true;
                }
                i += 1;
            }
            '+' => return Err(invalid(at, "+ isn't supported; use .* instead")),
            'C' => return Err(invalid(at, "C comes only at the start")),
            '\'' => {
                let close = chars[i + 1..].iter().position(|&c| c == '\'');
                let Some(len) = close else {
                    return Err(invalid(at, "This label has no closing '"));
                };
                let label: String = chars[i + 1..i + 1 + len].iter().collect();
                let Some(last) = steps.last_mut() else {
                    return Err(invalid(at, "A label follows an event"));
                };
                if last.grouped {
                    return Err(invalid(at, "Labels aren't allowed on [ ]"));
                }
                if last.star {
                    return Err(invalid(at, "Labels aren't allowed on .*"));
                }
                if last.label.is_some() {
                    return Err(invalid(at, "One label per event"));
                }
                if label.is_empty() {
                    return Err(invalid(at, "The label is empty"));
                }
                last.label = Some(label);
                i += len + 2;
            }
            '[' => {
                let mut events = 0;
                i += 1;
                loop {
                    let Some(&c) = chars.get(i) else {
                        return Err(invalid(at, "This [ has no closing ]"));
                    };
                    match c {
                        ']' => {
                            i += 1;
                            break;
                        }
                        '\'' => return Err(invalid(i + 1, "Labels aren't allowed inside [ ]")),
                        '.' => return Err(invalid(i + 1, ". isn't allowed inside [ ]")),
                        _ => match event(&chars, &mut i)? {
                            Some((_, Some(_))) => {
                                return Err(invalid(i, "Labels aren't allowed inside [ ]"));
                            }
                            Some((bits, None)) => events |= bits,
                            None => {
                                return Err(invalid(i + 1, format!("Unexpected {c} inside [ ]")));
                            }
                        },
                    }
                }
                if events == 0 {
                    return Err(invalid(at, "[ ] is empty"));
                }
                steps.push(Step {
                    events,
                    label: None,
                    star: false,
                    at,
                    grouped: true,
                });
            }
            c => match event(&chars, &mut i)? {
                Some((events, label)) => steps.push(Step {
                    events,
                    label,
                    star: false,
                    at,
                    grouped: false,
                }),
                None => return Err(invalid(at, format!("Unexpected {c}"))),
            },
        }
    }
    let end_at = chars.len() + 1;
    let Some(last) = steps.last() else {
        return Err(invalid(
            end_at,
            "Expected an event after C, e.g. .* or an end event (L, O, B or V)",
        ));
    };
    // Only the last event ends a path.
    for s in &steps[..steps.len() - 1] {
        if !s.star && s.events & SCATTER == 0 {
            return Err(invalid(
                s.at,
                "Nothing can follow an end event (L, O, B or V)",
            ));
        }
    }
    if !last.star && last.events & END == 0 {
        return Err(invalid(end_at, "Expected an end event (L, O, B or V)"));
    }
    Ok(steps)
}

/// The expression, normalised: scatter events before `.*`, whether there
/// is one, scatter events after it, and the end event with its label.
#[derive(Debug, PartialEq, Eq)]
struct Path {
    before: Vec<Step>,
    star: bool,
    after: Vec<Step>,
    end: u8,
    end_label: Option<String>,
}

fn normalise(mut steps: Vec<Step>) -> Path {
    let last = steps.pop().expect("parse checks there is one");
    let (mut before, mut after, mut star) = (Vec::new(), Vec::new(), false);
    // A trailing .* is "any number of bounces, then any end".
    let trailing_star = last.star;
    let (end, end_label) = if trailing_star {
        (END, None)
    } else {
        (last.events & END, last.label)
    };
    for mut s in steps {
        if s.star {
            star = true;
            continue;
        }
        s.events &= SCATTER;
        if star {
            after.push(s);
        } else {
            before.push(s);
        }
    }
    Path {
        before,
        star: star || trailing_star,
        after,
        end,
        end_label,
    }
}

/// What `text` renders as.
pub fn recipe(text: &str) -> Recipe {
    let path = match parse(text) {
        Ok(steps) => normalise(steps),
        Err(status) => {
            return Recipe {
                terms: Vec::new(),
                status,
                fallback: None,
            };
        }
    };
    match map(&path) {
        Ok((terms, status)) => Recipe {
            terms,
            status,
            fallback: None,
        },
        Err(why) => Recipe {
            terms: Vec::new(),
            status: Status::Unsupported(why.into()),
            fallback: Some(fallback(&path)),
        },
    }
}

/// The nearest exact preset to an unsupported expression.
fn fallback(path: &Path) -> String {
    if let Some(label) = &path.end_label
        && !label.contains('\'')
    {
        return per_light_expression(label);
    }
    match path.before.first().map(|s| s.events) {
        Some(e) if e & RS != 0 && e & (RD | TD) == 0 => "C<RS>.*".into(),
        Some(e) if e & TS != 0 && e & (RD | TD | RS) == 0 => "C<TS>.*".into(),
        Some(_) => "C<RD>.*".into(),
        None => "C.*".into(),
    }
}

/// The families covering scatter events `x`, and the events they hold.
fn families(x: u8) -> (Vec<Scatter>, u8) {
    let mut out = Vec::new();
    let mut covered = 0;
    if x & (RD | TD) != 0 {
        out.push(Scatter::Diffuse);
        covered |= RD | TD;
    }
    if x & RS != 0 {
        out.push(Scatter::Glossy);
        covered |= RS;
    }
    if x & TS != 0 {
        out.push(Scatter::Transmission);
        covered |= TS;
    }
    (out, covered)
}

type Mapped = Result<(Vec<Term>, Status), &'static str>;

/// The mapping table (spec §2, RO-Q23..Q26).
fn map(path: &Path) -> Mapped {
    if path
        .before
        .iter()
        .chain(&path.after)
        .any(|s| s.label.is_some())
    {
        return Err("labels on surfaces (like <RD'skin'>) have no pass");
    }
    if !path.after.is_empty() {
        return Err("events after .* (like the last bounce) have no pass");
    }
    if path.before.is_empty() {
        if path.star {
            whole_paths(path)
        } else {
            seen_directly(path)
        }
    } else {
        bounced(path)
    }
}

/// `C.*X`: every path ending at X.
fn whole_paths(path: &Path) -> Mapped {
    if let Some(label) = &path.end_label {
        if path.end & O != 0 {
            return Err("glowing surfaces have no LPE Label");
        }
        if path.end & V != 0 {
            return Err("volumes have no LPE Label");
        }
        // Lights and the environment share labels: one group (RO-Q25).
        return Ok((
            vec![Term::Group(Group::Label(label.clone()))],
            Status::Exact,
        ));
    }
    if path.end == END {
        return Ok((vec![Term::Combined], Status::Exact));
    }
    if path.end & V != 0 {
        return Err("light ending in a volume on its own (C.*V) has no pass");
    }
    let mut terms = Vec::new();
    if path.end & L != 0 {
        terms.push(Term::Group(Group::AllLights));
    }
    if path.end & O != 0 {
        terms.push(Term::Group(Group::Emitters));
    }
    if path.end & B != 0 {
        terms.push(Term::Group(Group::World));
    }
    Ok((terms, Status::Exact))
}

/// `CX`: emitters seen directly by the camera.
fn seen_directly(path: &Path) -> Mapped {
    if path.end_label.is_some() {
        return Err("labels work on whole-path expressions like C.*L'key'");
    }
    let mut terms = Vec::new();
    let mut status = Status::Exact;
    if path.end & B != 0 {
        terms.push(Term::Background);
    }
    if path.end & (L | O) != 0 {
        terms.push(Term::Emission);
        if path.end & O == 0 {
            // RO-Q24: lamps the camera sees land in the Emission pass with
            // the glowing surfaces.
            status = Status::Approximate(
                "lamps seen directly share the Emission pass with glowing surfaces".into(),
            );
        }
    }
    if path.end & V != 0 {
        terms.push(Term::Volume(Part::Both));
        status = Status::Approximate("the volume passes (scattering at the first hit)".into());
    }
    Ok((terms, status))
}

/// `C<X>...`: light that bounced off a surface first.
fn bounced(path: &Path) -> Mapped {
    if path.end_label.is_some() {
        return Err("light groups can't be split by surface type (like C<RS>L'key')");
    }
    if path.end & EMITTERS == 0 {
        return Err("volumes after a bounce have no pass");
    }
    let first = path.before[0].events;
    if path.before[1..].iter().any(|s| s.events != SCATTER) {
        return Err("events in the middle of a path (like caustics, CDS.*) have no pass");
    }
    let part = match (path.before.len() - 1, path.star) {
        (0, false) => Part::Direct,
        (0, true) => Part::Both,
        (1, true) => Part::Indirect,
        _ => return Err("an exact number of bounces after the first has no pass"),
    };
    if first == TD {
        return Err("diffuse transmission (translucency) is inside the diffuse pass");
    }
    let (fams, covered) = families(first);
    let mut status = Status::Exact;
    // RO-Q23: <RD> counts as exact although translucency and subsurface
    // sit inside the diffuse pass; wider letters that take in only part
    // of the diffuse pass are approximate.
    if covered != first && first != RD {
        status = Status::Approximate(
            "the diffuse pass also holds diffuse transmission (translucency)".into(),
        );
    }
    // RO-Q26: the light passes count lamps, glowing surfaces and the
    // environment together; L (or any end) reads as all of them.
    if path.end != END && path.end & EMITTERS != EMITTERS && path.end & EMITTERS != L {
        status = Status::Approximate(
            "the light passes count lamps, glowing surfaces and the environment together".into(),
        );
    }
    let mut terms: Vec<Term> = fams.into_iter().map(|f| Term::Shading(f, part)).collect();
    if first == SCATTER {
        terms.push(Term::Volume(part));
    }
    Ok((terms, status))
}

#[cfg(test)]
mod tests {
    use super::*;
    use Part::*;
    use Scatter::*;

    fn terms(text: &str) -> Vec<Term> {
        let r = recipe(text);
        assert!(r.status.renders(), "{text}: {:?}", r.status);
        r.terms
    }

    #[test]
    fn every_preset_parses() {
        for (name, text) in PRESETS {
            let r = recipe(text);
            assert!(
                !matches!(r.status, Status::Invalid { .. } | Status::Empty),
                "{name}: {:?}",
                r.status
            );
        }
        // Caustics is the one the renderer can't give.
        assert!(matches!(recipe("CDS.*").status, Status::Unsupported(_)));
        let labels = vec!["Key".to_owned(), "Rim".to_owned(), "it's".to_owned()];
        let scene = label_presets(&labels);
        assert_eq!(scene[0], ("All Lights".into(), "C.*L".into()));
        assert_eq!(scene[1], ("Light: Key".into(), "C.*L'Key'".into()));
        assert_eq!(scene.len(), 3, "a label with a quote is left out");
        for (_, text) in &scene {
            assert_eq!(recipe(text).status, Status::Exact, "{text}");
        }
        assert!(label_presets(&[]).is_empty());
    }

    #[test]
    fn the_mapping_table() {
        assert_eq!(terms("C.*"), [Term::Combined]);
        assert_eq!(terms("CB"), [Term::Background]);
        assert_eq!(terms("CO"), [Term::Emission]);
        assert_eq!(terms("CV"), [Term::Volume(Both)]);
        assert!(matches!(recipe("CV").status, Status::Approximate(_)));
        for t in ["C<RD>L", "CDL"] {
            assert_eq!(terms(t), [Term::Shading(Diffuse, Direct)], "{t}");
            assert_eq!(recipe(t).status, Status::Exact, "{t}");
        }
        assert_eq!(
            terms("CRL"),
            [
                Term::Shading(Diffuse, Direct),
                Term::Shading(Glossy, Direct)
            ]
        );
        assert!(matches!(recipe("CRL").status, Status::Approximate(_)));
        assert_eq!(terms("C<RD>.*"), [Term::Shading(Diffuse, Both)]);
        assert_eq!(terms("CD.*"), [Term::Shading(Diffuse, Both)]);
        assert_eq!(terms("C<RD>..*"), [Term::Shading(Diffuse, Indirect)]);
        assert_eq!(terms("C<RS>L"), [Term::Shading(Glossy, Direct)]);
        assert_eq!(terms("C<RS>.*"), [Term::Shading(Glossy, Both)]);
        assert_eq!(terms("C<TS>L"), [Term::Shading(Transmission, Direct)]);
        assert_eq!(terms("C<TS>.*"), [Term::Shading(Transmission, Both)]);
        assert_eq!(
            terms("CS.*"),
            [
                Term::Shading(Glossy, Both),
                Term::Shading(Transmission, Both)
            ]
        );
        assert_eq!(recipe("CS.*").status, Status::Exact);
        assert_eq!(
            terms("C.L"),
            [
                Term::Shading(Diffuse, Direct),
                Term::Shading(Glossy, Direct),
                Term::Shading(Transmission, Direct),
                Term::Volume(Direct)
            ]
        );
        assert_eq!(terms("C.*L"), [Term::Group(Group::AllLights)]);
        assert_eq!(
            terms("C.*L'key'"),
            [Term::Group(Group::Label("key".into()))]
        );
        assert_eq!(
            terms("C.*B'sky'"),
            [Term::Group(Group::Label("sky".into()))]
        );
        assert_eq!(
            terms("C.*[OB]"),
            [Term::Group(Group::Emitters), Term::Group(Group::World)]
        );
        // The ones the renderer can't give: black, with a reason and the
        // nearest exact preset.
        for (t, near) in [
            ("C<TD>.*", "C<RD>.*"),
            ("C<RS>L'key'", "C.*L'key'"),
            ("CDS.*", "C<RD>.*"),
            ("C<RD'skin'>L", "C<RD>.*"),
            ("C.*O'neon'", "C.*L'neon'"),
            ("C.*<RD>L", "C.*"),
            ("C<RD>.L", "C<RD>.*"),
        ] {
            let r = recipe(t);
            assert!(
                matches!(r.status, Status::Unsupported(_)),
                "{t}: {:?}",
                r.status
            );
            assert!(r.terms.is_empty() && !r.status.renders(), "{t}");
            assert_eq!(r.fallback.as_deref(), Some(near), "{t}");
            assert!(
                r.status
                    .line()
                    .starts_with("Not supported by the renderer: ")
            );
        }
    }

    #[test]
    fn dot_and_star_rules() {
        assert!(recipe("C .* ").status.renders(), "spaces are ignored");
        assert!(recipe("C<..>L").status.renders(), "<..> is any scatter");
        for (t, at, msg) in [
            ("C*", 2, "* only follows ."),
            ("CL*", 3, "* only follows ."),
            ("C.*.*", 5, "Only one .*"),
            ("C.*'x'", 4, "Labels aren't allowed on .*"),
            ("C.+", 3, "+ isn't supported"),
            ("C[L'k']", 4, "Labels aren't allowed inside [ ]"),
            ("C[.L]", 3, ". isn't allowed inside [ ]"),
            ("C<RD>", 6, "Expected an end event"),
            ("C", 2, "Expected an event after C"),
            ("LC", 1, "starts with C"),
            ("CLD", 2, "Nothing can follow an end event"),
            ("C<RX>L", 2, "Expected <RD>"),
            ("C.*L'key", 5, "no closing '"),
            ("C[OB", 2, "no closing ]"),
            ("CQ", 2, "Unexpected Q"),
            ("CC", 2, "C comes only at the start"),
        ] {
            match recipe(t).status {
                Status::Invalid { at: got, message } => {
                    assert_eq!(got, at, "{t}: {message}");
                    assert!(message.contains(msg), "{t}: {message}");
                }
                other => panic!("{t}: {other:?}"),
            }
        }
        assert_eq!(recipe("").status, Status::Empty);
        assert_eq!(recipe("  ").status.line(), "Type an expression");
        assert!(recipe("CQ").status.line().ends_with("(at character 2)"));
    }
}

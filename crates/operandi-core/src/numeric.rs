use crate::Result;
/// Small bounded recursive descent parser. Distances normalize to meters;
/// arithmetic supports explicit units on terms (e.g. `1 m/4`, `250 mm`).
#[derive(Clone, Copy)]
pub enum Quantity {
    Distance,
    Angle,
    Scalar,
    Percent,
}
pub fn parse(text: &str) -> Result<f64> {
    parse_quantity(text, Quantity::Distance)
}
pub fn parse_quantity(text: &str, quantity: Quantity) -> Result<f64> {
    if text.len() > 256 {
        return Err("Expression is too long".into());
    }
    let mut p = Parser {
        s: text.as_bytes(),
        i: 0,
        depth: 0,
        quantity,
        units: crate::units::current(),
        saw_unit: false,
    };
    let mut v = p.expr()?;
    p.ws();
    if p.i != p.s.len() || !v.is_finite() {
        return Err(
            "Enter a finite number or expression with m, cm, mm, µm, km, in, ft, yd, mi or gu"
                .into(),
        );
    }
    // A distance typed with no unit at all is in the Default Unit; once any
    // term has a unit, plain numbers are factors (`1 m/4`).
    if matches!(quantity, Quantity::Distance) && !p.saw_unit {
        v *= p.units.default_unit.metres(&p.units);
    }
    Ok(v)
}
struct Parser<'a> {
    s: &'a [u8],
    i: usize,
    depth: usize,
    quantity: Quantity,
    units: crate::units::DistanceUnits,
    saw_unit: bool,
}
impl Parser<'_> {
    fn ws(&mut self) {
        while self.i < self.s.len() && self.s[self.i].is_ascii_whitespace() {
            self.i += 1;
        }
    }
    fn eat(&mut self, c: u8) -> bool {
        self.ws();
        if self.s.get(self.i) == Some(&c) {
            self.i += 1;
            true
        } else {
            false
        }
    }
    fn expr(&mut self) -> Result<f64> {
        let mut v = self.term()?;
        loop {
            if self.eat(b'+') {
                v += self.term()?;
            } else if self.eat(b'-') {
                v -= self.term()?;
            } else {
                break;
            }
        }
        Ok(v)
    }
    fn term(&mut self) -> Result<f64> {
        let mut v = self.atom()?;
        loop {
            if self.eat(b'*') {
                v *= self.atom()?;
            } else if self.eat(b'/') {
                let d = self.atom()?;
                if d == 0. {
                    return Err("Division by zero".into());
                }
                v /= d;
            } else {
                break;
            }
        }
        Ok(v)
    }
    fn atom(&mut self) -> Result<f64> {
        self.ws();
        if self.eat(b'-') {
            return Ok(-self.atom()?);
        }
        if self.eat(b'+') {
            return self.atom();
        }
        self.depth += 1;
        if self.depth > 32 {
            return Err("Expression nesting limit".into());
        }
        let mut v = if self.eat(b'(') {
            let n = self.expr()?;
            if !self.eat(b')') {
                return Err("Missing closing parenthesis".into());
            }
            n
        } else {
            let start = self.i;
            while self.i < self.s.len()
                && (self.s[self.i].is_ascii_digit() || self.s[self.i] == b'.')
            {
                self.i += 1;
            }
            if self.i == start {
                return Err("Expected a number".into());
            }
            std::str::from_utf8(&self.s[start..self.i])
                .map_err(|_| "Invalid text")?
                .parse::<f64>()
                .map_err(|_| "Invalid number")?
        };
        self.ws();
        let start = self.i;
        while self.i < self.s.len()
            && (self.s[self.i].is_ascii_alphabetic()
                || self.s[self.i] >= 128
                || self.s[self.i] == b'%')
        {
            self.i += 1;
        }
        // Feet and inch marks: 3' 6" (English units).
        if self.i == start && matches!(self.s.get(self.i), Some(b'\'' | b'"')) {
            self.i += 1;
        }
        let unit = std::str::from_utf8(&self.s[start..self.i]).map_err(|_| "Invalid unit")?;
        if matches!(self.quantity, Quantity::Distance) && !unit.is_empty() {
            self.saw_unit = true;
        }
        v *= match (self.quantity, unit) {
            (_, "") => 1.,
            (Quantity::Distance, "m") => 1.,
            (Quantity::Distance, "cm") => 0.01,
            (Quantity::Distance, "mm") => 0.001,
            (Quantity::Distance, "µm" | "um") => 1e-6,
            (Quantity::Distance, "km") => 1000.,
            (Quantity::Distance, "in" | "\"") => 0.0254,
            (Quantity::Distance, "mil") => 0.0000254,
            (Quantity::Distance, "ft" | "'") => 0.3048,
            (Quantity::Distance, "yd") => 0.9144,
            (Quantity::Distance, "mi") => 1609.344,
            (Quantity::Distance, "gu") => self.units.meters_per_game_unit,
            (Quantity::Percent, "%") => 1.,
            (Quantity::Angle, "deg" | "°") => 1.,
            (Quantity::Angle, "rad") => 180. / std::f64::consts::PI,
            _ => return Err("Unit is incompatible with this field".into()),
        };
        // 3' 6": inches right after feet add to them.
        if unit == "'" {
            self.ws();
            if self.s.get(self.i).is_some_and(u8::is_ascii_digit) {
                v += self.atom()?;
            }
        }
        self.depth -= 1;
        Ok(v)
    }
}

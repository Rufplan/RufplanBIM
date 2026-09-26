//! A small ISO 10303-21 (STEP physical file) reader for IFC import (ADR-035): the DATA
//! section as entities by instance number, with typed argument values.

use std::collections::HashMap;

/// An argument value.
#[derive(Debug, Clone, PartialEq)]
pub enum V {
    /// `$`: unset.
    Null,
    /// `*`: derived.
    Star,
    Ref(u64),
    Str(String),
    /// `.ENUM.` (and `.T.` / `.F.`), without the dots.
    Enum(String),
    Num(f64),
    List(Vec<V>),
    /// A typed value such as `IFCLABEL('x')` or `IFCLENGTHMEASURE(3.)`.
    Typed(String, Vec<V>),
}

impl V {
    pub fn num(&self) -> Option<f64> {
        match self {
            V::Num(n) => Some(*n),
            V::Typed(_, a) => a.first()?.num(),
            _ => None,
        }
    }
    pub fn str(&self) -> Option<&str> {
        match self {
            V::Str(s) => Some(s),
            V::Typed(_, a) => a.first()?.str(),
            _ => None,
        }
    }
    pub fn r(&self) -> Option<u64> {
        match self {
            V::Ref(r) => Some(*r),
            _ => None,
        }
    }
    pub fn list(&self) -> &[V] {
        match self {
            V::List(l) => l,
            _ => &[],
        }
    }
    pub fn enumv(&self) -> Option<&str> {
        match self {
            V::Enum(e) => Some(e),
            _ => None,
        }
    }
    pub fn refs(&self) -> Vec<u64> {
        self.list().iter().filter_map(V::r).collect()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Entity {
    /// Upper-case type name, e.g. `IFCWALL`.
    pub name: String,
    pub args: Vec<V>,
}

impl Entity {
    pub fn arg(&self, i: usize) -> &V {
        self.args.get(i).unwrap_or(&V::Null)
    }
}

/// The file's schema (`IFC2X3`, `IFC4`…) and entities.
#[derive(Debug, Default)]
pub struct StepFile {
    pub schema: String,
    pub entities: HashMap<u64, Entity>,
}

impl StepFile {
    pub fn get(&self, id: u64) -> Option<&Entity> {
        self.entities.get(&id)
    }
    /// The entity `v` refers to.
    pub fn at(&self, v: &V) -> Option<&Entity> {
        self.get(v.r()?)
    }
    /// Instance numbers of every entity named `name`, in file order.
    pub fn all(&self, name: &str) -> Vec<u64> {
        let mut v: Vec<u64> = self
            .entities
            .iter()
            .filter(|(_, e)| e.name == name)
            .map(|(k, _)| *k)
            .collect();
        v.sort_unstable();
        v
    }
}

struct Cursor<'a> {
    b: &'a [u8],
    i: usize,
}

impl Cursor<'_> {
    fn ws(&mut self) {
        loop {
            while self.i < self.b.len() && self.b[self.i].is_ascii_whitespace() {
                self.i += 1;
            }
            // Comments /* … */.
            if self.b.get(self.i) == Some(&b'/') && self.b.get(self.i + 1) == Some(&b'*') {
                self.i += 2;
                while self.i + 1 < self.b.len()
                    && !(self.b[self.i] == b'*' && self.b[self.i + 1] == b'/')
                {
                    self.i += 1;
                }
                self.i += 2;
                continue;
            }
            break;
        }
    }
    fn peek(&self) -> Option<u8> {
        self.b.get(self.i).copied()
    }
    fn ident(&mut self) -> String {
        let s = self.i;
        while self.i < self.b.len()
            && (self.b[self.i].is_ascii_alphanumeric() || self.b[self.i] == b'_')
        {
            self.i += 1;
        }
        String::from_utf8_lossy(&self.b[s..self.i]).to_ascii_uppercase()
    }
    fn args(&mut self) -> Vec<V> {
        let mut out = vec![];
        self.ws();
        if self.peek() != Some(b'(') {
            return out;
        }
        self.i += 1;
        loop {
            self.ws();
            match self.peek() {
                Some(b')') => {
                    self.i += 1;
                    break;
                }
                Some(b',') => {
                    self.i += 1;
                }
                None => break,
                _ => out.push(self.value()),
            }
        }
        out
    }
    fn value(&mut self) -> V {
        self.ws();
        match self.peek() {
            Some(b'$') => {
                self.i += 1;
                V::Null
            }
            Some(b'*') => {
                self.i += 1;
                V::Star
            }
            Some(b'#') => {
                self.i += 1;
                let s = self.i;
                while self.i < self.b.len() && self.b[self.i].is_ascii_digit() {
                    self.i += 1;
                }
                V::Ref(
                    std::str::from_utf8(&self.b[s..self.i])
                        .ok()
                        .and_then(|t| t.parse().ok())
                        .unwrap_or(0),
                )
            }
            Some(b'\'') => {
                self.i += 1;
                let mut raw = vec![];
                while self.i < self.b.len() {
                    let c = self.b[self.i];
                    self.i += 1;
                    if c == b'\'' {
                        if self.peek() == Some(b'\'') {
                            raw.push(b'\'');
                            self.i += 1;
                            continue;
                        }
                        break;
                    }
                    raw.push(c);
                }
                V::Str(decode(&raw))
            }
            Some(b'.') => {
                self.i += 1;
                let s = self.i;
                while self.i < self.b.len() && self.b[self.i] != b'.' {
                    self.i += 1;
                }
                let e = String::from_utf8_lossy(&self.b[s..self.i]).to_string();
                self.i += 1;
                V::Enum(e)
            }
            Some(b'(') => V::List(self.args()),
            Some(c) if c == b'-' || c == b'+' || c.is_ascii_digit() => {
                let s = self.i;
                self.i += 1;
                while self.i < self.b.len()
                    && (self.b[self.i].is_ascii_digit()
                        || matches!(self.b[self.i], b'.' | b'E' | b'e' | b'-' | b'+'))
                {
                    self.i += 1;
                }
                V::Num(
                    std::str::from_utf8(&self.b[s..self.i])
                        .ok()
                        .and_then(|t| t.parse().ok())
                        .unwrap_or(0.0),
                )
            }
            Some(c) if c.is_ascii_alphabetic() => {
                let name = self.ident();
                V::Typed(name, self.args())
            }
            _ => {
                self.i += 1;
                V::Null
            }
        }
    }
}

/// Decodes STEP string escapes: `\X2\…\X0\` (UTF-16 hex), `\X\hh`, `\S\c` and `\\`.
fn decode(raw: &[u8]) -> String {
    let s = String::from_utf8_lossy(raw).to_string();
    if !s.contains('\\') {
        return s;
    }
    let mut out = String::new();
    let mut rest = s.as_str();
    while let Some(p) = rest.find('\\') {
        out.push_str(&rest[..p]);
        rest = &rest[p..];
        if let Some(r) = rest.strip_prefix("\\X2\\") {
            let end = r.find("\\X0\\").unwrap_or(r.len());
            let units: Vec<u16> = r.as_bytes()[..end]
                .chunks(4)
                .filter_map(|c| u16::from_str_radix(std::str::from_utf8(c).ok()?, 16).ok())
                .collect();
            out.push_str(&String::from_utf16_lossy(&units));
            rest = r.get(end + 4..).unwrap_or("");
        } else if let Some(r) = rest.strip_prefix("\\X\\") {
            if let Some(b) = r.get(..2).and_then(|h| u8::from_str_radix(h, 16).ok()) {
                out.push(char::from(b));
            }
            rest = r.get(2..).unwrap_or("");
        } else if let Some(r) = rest.strip_prefix("\\S\\") {
            if let Some(c) = r.chars().next() {
                out.push(char::from_u32(c as u32 + 128).unwrap_or(c));
                rest = &r[c.len_utf8()..];
            } else {
                rest = "";
            }
        } else if let Some(r) = rest.strip_prefix("\\\\") {
            out.push('\\');
            rest = r;
        } else {
            out.push('\\');
            rest = &rest[1..];
        }
    }
    out.push_str(rest);
    out
}

/// Parses a STEP file.
pub fn parse(text: &str) -> Result<StepFile, String> {
    let schema = text
        .find("FILE_SCHEMA")
        .and_then(|i| {
            let r = &text[i..];
            let a = r.find('\'')? + 1;
            let b = r[a..].find('\'')? + a;
            Some(r[a..b].to_ascii_uppercase())
        })
        .unwrap_or_default();
    let start = text
        .find("DATA;")
        .ok_or("not an IFC (STEP) file: no DATA section")?
        + 5;
    let mut c = Cursor {
        b: text.as_bytes(),
        i: start,
    };
    let mut entities = HashMap::new();
    loop {
        c.ws();
        match c.peek() {
            Some(b'#') => {
                c.i += 1;
                let s = c.i;
                while c.i < c.b.len() && c.b[c.i].is_ascii_digit() {
                    c.i += 1;
                }
                let id: u64 = std::str::from_utf8(&c.b[s..c.i])
                    .ok()
                    .and_then(|t| t.parse().ok())
                    .unwrap_or(0);
                c.ws();
                if c.peek() == Some(b'=') {
                    c.i += 1;
                }
                c.ws();
                let name = c.ident();
                let args = c.args();
                entities.insert(id, Entity { name, args });
                // To the end of the instance.
                while c.i < c.b.len() && c.b[c.i] != b';' {
                    c.i += 1;
                }
                c.i += 1;
            }
            Some(b'E') if text[c.i..].starts_with("ENDSEC") => break,
            None => break,
            _ => c.i += 1,
        }
    }
    if entities.is_empty() {
        return Err("the file has no entities".into());
    }
    Ok(StepFile { schema, entities })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_entities_and_values() {
        let f = parse(
            "ISO-10303-21;\nHEADER;FILE_SCHEMA(('IFC4'));ENDSEC;\nDATA;\n#1=IFCCARTESIANPOINT((0.,-1.5E3,2.));\n#2= IFCWALL('g',$,'Wall \\X2\\00E9\\X0\\ ''A''',*,.T.,(#1,#3),IFCLABEL('x'));\n/* c */#3=IFCDIRECTION((1.,0.));\nENDSEC;END-ISO-10303-21;",
        )
        .unwrap();
        assert_eq!(f.schema, "IFC4");
        assert_eq!(f.entities.len(), 3);
        let p = f.get(1).unwrap();
        assert_eq!(p.arg(0).list()[1], V::Num(-1500.0));
        let w = f.get(2).unwrap();
        assert_eq!(w.name, "IFCWALL");
        assert_eq!(w.arg(2).str(), Some("Wall é 'A'"));
        assert_eq!((w.arg(1), w.arg(3)), (&V::Null, &V::Star));
        assert_eq!(w.arg(4).enumv(), Some("T"));
        assert_eq!(w.arg(5).refs(), vec![1, 3]);
        assert_eq!(w.arg(6).str(), Some("x"));
        assert_eq!(f.all("IFCDIRECTION"), vec![3]);
    }
}

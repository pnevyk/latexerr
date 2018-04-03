use regex::Regex;

pub struct PatternBuilder {
    pattern: String,
}

impl PatternBuilder {
    pub fn new() -> Self {
        Self {
            pattern: String::new(),
        }
    }

    pub fn raw(self, pattern: &str) -> Self {
        Self {
            pattern: self.pattern + pattern,
        }
    }

    pub fn line(self, pattern: &str) -> Self {
        self.raw(&(pattern.to_owned() + r"\n"))
    }

    pub fn error(self, pattern: &str) -> Self {
        self.line(&("! ".to_owned() + pattern))
    }

    pub fn any_on_line(self) -> Self {
        self.line(".*")
    }

    pub fn location(self) -> Self {
        self.raw(r"l\.(\d+) ")
    }

    pub fn location_with_arg(self) -> Self {
        self.raw(r"l\.(\d+) (.+)")
    }
}

impl Into<Regex> for PatternBuilder {
    fn into(self) -> Regex {
        Regex::new(&self.pattern).unwrap()
    }
}

use std::fmt;
use std::str::FromStr;

use regex::{Captures, Regex};
use yansi::Paint;

use utils::PatternBuilder;

#[derive(PartialEq, Eq, Hash)]
pub enum Location {
    Line(usize),
    End,
    None,
}

pub enum LogItemTypeLevel {
    Error,
    Warning,
}

impl fmt::Display for LogItemTypeLevel {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            LogItemTypeLevel::Error => write!(f, "{}", Paint::red("Error")),
            LogItemTypeLevel::Warning => write!(f, "{}", Paint::yellow("Warning")),
        }
    }
}

#[derive(PartialEq, Eq, Hash)]
pub enum LogItemType<'a> {
    /// When there is used a control sequence which is undefined.
    ///
    /// Example latex source:
    /// ```latex
    /// \documentclass{article}
    ///
    /// \begin{document}
    /// \foo
    /// \end{document}
    /// ```
    ///
    /// Example log output:
    /// ```txt
    /// ! Undefined control sequence.
    /// l.4 \foo
    /// ```
    UndefinedControlSequence(&'a str),

    /// When there is missing starting brace for a command.
    ///
    /// Example latex source:
    /// ```latex
    /// \documentclass{article}
    ///
    /// \begin{document}
    /// \date April 2018}
    /// \end{document}
    /// ```
    ///
    /// Example log output:
    /// ```txt
    /// ! Too many }'s.
    /// l.4 \date April 2018}
    /// ```
    TooManyEndingBraces(&'a str),

    /// When a string, which is valid only in math mode, is used outside math environments.
    ///
    /// Example latex source:
    /// ```latex
    /// \documentclass{article}
    ///
    /// \begin{document}
    /// _
    /// \end{document}
    /// ```
    ///
    /// Example log output:
    /// ```txt
    /// ! Missing $ inserted.
    /// <inserted text>
    ///                 $
    /// l.4 _
    /// ```
    NotInMathMode(&'a str),

    /// When there is missing ending brace for a command.
    ///
    /// Example latex source:
    /// ```latex
    /// \documentclass{article}
    ///
    /// \begin{document}
    /// \date{April 2018 \maketitle
    ///
    /// \date{April 2018 \maketitle
    /// \end{document}
    /// ```
    ///
    /// Example log output:
    /// ```txt
    /// Runaway argument?
    /// {April 2018 \maketitle
    /// ! Paragraph ended before \date was complete.
    /// <to be read again>
    ///                    \par
    /// l.5
    ///
    /// Runaway argument?
    /// {April 2018 \maketitle \end {document}
    /// ! File ended while scanning use of \date.
    /// <inserted text>
    ///                 \par
    /// <*> runaway_argument.tex
    /// ```
    RunawayArgument(&'a str),

    /// When a line cannot be stretched to fit.
    ///
    /// Example latex source:
    /// ```latex
    /// \documentclass{article}
    ///
    /// \begin{document}
    ///
    /// Donec nec sapien scelerisque, sagittis augue dictum, interdum nisl. \\ \\ Aenean est libero, porttitor vitae mi non, sodales mattis lectus.
    ///
    /// \end{document}
    /// ```
    ///
    /// Example log output:
    /// ```txt
    /// Underfull \hbox (badness 10000) in paragraph at lines 5--6
    ///
    ///  []
    /// ```
    UnderfullHBox(&'a str, usize),

    /// When a line overflows maximum width.
    ///
    /// Example latex source:
    /// ```latex
    /// \documentclass{article}
    ///
    /// \begin{document}
    ///
    /// Lorem ipsum dolor sit amet, consectetur adipiscing elit. Sed in \({e^{ix} = \cos(x) + i \sin(x)}\) condimentum erat.
    ///
    /// \end{document}
    /// ```
    ///
    /// Example log output:
    /// ```txt
    /// Overfull \hbox (35.0259pt too wide) in paragraph at lines 5--6
    /// []\OT1/cmr/m/n/10 Lorem ip-sum do-lor sit amet, con-secte-tur adip-isc-ing elit
    /// . Sed in $[]$
    ///  []
    /// ```
    OverfullHBox(&'a str),

    /// When a unknown package is tried to be used.
    ///
    /// Example latex source:
    /// ```latex
    /// \documentclass{article}
    ///
    /// \usepackage{missing}
    ///
    /// \begin{document}
    /// \end{document}
    /// ```
    ///
    /// Example log output:
    /// ```txt
    /// ! LaTeX Error: File `missing.sty' not found.
    /// ```
    MissingPackage(&'a str),

    /// When an invalid option is passed into a package.
    ///
    /// Example latex source:
    /// ```latex
    /// \documentclass{article}
    ///
    /// \usepackage[invalid]{graphicx}
    ///
    /// \begin{document}
    /// \end{document}
    /// ```
    ///
    /// Example log output:
    /// ```txt
    /// ! LaTeX Error: Unknown option `invalid' for package `graphics'.
    /// ```
    InvalidOption(&'a str, &'a str),

    /// When too many &'s are in a row of a table, array or eqnarray.
    ///
    /// Example of latex source:
    /// ```latex
    /// \documentclass{article}
    ///
    /// \begin{document}
    ///
    /// \begin{table}
    ///   \begin{tabular}{c}
    ///     Foo & Bar \\
    ///   \end{tabular}
    /// \end{table}
    ///
    /// \end{document}
    /// ```
    ///
    /// Example log output:
    /// ```txt
    /// ! Extra alignment tab has been changed to \cr.
    /// <recently read> \endtemplate
    ///
    /// l.7     Foo &
    ///               Bar \\
    /// ```
    ExtraAlignmentToCR(&'a str),
}

impl<'a> LogItemType<'a> {
    fn get_level(&self) -> LogItemTypeLevel {
        match *self {
            LogItemType::UndefinedControlSequence(_)
            | LogItemType::TooManyEndingBraces(_)
            | LogItemType::NotInMathMode(_)
            | LogItemType::RunawayArgument(_)
            | LogItemType::MissingPackage(_)
            | LogItemType::InvalidOption(_, _)
            | LogItemType::ExtraAlignmentToCR(_) => LogItemTypeLevel::Error,
            LogItemType::UnderfullHBox(_, _) | LogItemType::OverfullHBox(_) => {
                LogItemTypeLevel::Warning
            }
        }
    }
}

#[derive(PartialEq, Eq, Hash)]
pub struct LogItem<'a> {
    pub item_type: LogItemType<'a>,
    pub location: Location,
}

impl<'a> LogItem<'a> {
    fn new(item_type: LogItemType<'a>, location: Location) -> Self {
        Self {
            item_type,
            location,
        }
    }

    pub fn rules() -> Vec<&'a Rule<'a>> {
        vec![
            &UndefinedControlSequence,
            &TooManyEndingBraces,
            &NotInMathMode,
            &RunawayArgument,
            &RunawayArgument2,
            &UnderfullHBox,
            &OverfullHBox,
            &MissingPackage,
            &InvalidOption,
            &ExtraAlignmentToCR,
        ]
    }
}

impl<'a> fmt::Display for LogItem<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let level = self.item_type.get_level();

        match self.location {
            Location::Line(line) => write!(
                f,
                "{} {} {}: ",
                level,
                Paint::white("on line").italic(),
                Paint::white(line).bold()
            )?,
            Location::End => write!(f, "{} {}: ", level, Paint::white("at the end").italic())?,
            Location::None => write!(f, "{}: ", level)?,
        }

        match self.item_type {
            LogItemType::UndefinedControlSequence(command) => {
                write!(f, "Unknown command {}.", Paint::cyan(command))
            }
            LogItemType::TooManyEndingBraces(context) => write!(
                f,
                "Number of curly braces near {} does not match.",
                Paint::white(context).bold()
            ),
            LogItemType::NotInMathMode(input) => write!(
                f,
                "String {} is valid only in math mode.",
                Paint::white(input).bold()
            ),
            LogItemType::RunawayArgument(command) => write!(
                f,
                "Command {} was not properly ended with curly brace.",
                Paint::cyan(command),
            ),
            LogItemType::UnderfullHBox(input, badness) => if input.is_empty() {
                write!(
                    f,
                    "Line cannot be stretch enough. The problem is {}.",
                    if badness < 2000 {
                        "ignorable"
                    } else if badness < 6000 {
                        "not as bad"
                    } else {
                        "very bad"
                    }
                )
            } else {
                write!(
                    f,
                    "Due to {} the line cannot be stretch enough. The problem is {}.",
                    input,
                    if badness < 2000 {
                        "ignorable"
                    } else if badness < 6000 {
                        "not as bad"
                    } else {
                        "very bad"
                    }
                )
            },
            LogItemType::OverfullHBox(input) => write!(
                f,
                "Text after {} (displayed hyphenated) overflows the line end.",
                Paint::white(input.replace("\n", "").trim()).bold()
            ),
            LogItemType::MissingPackage(package) => {
                write!(f, "Missing package {}.", Paint::cyan(package))
            }
            LogItemType::InvalidOption(option, package) => write!(
                f,
                "Invalid option {} of package {}.",
                Paint::cyan(option),
                Paint::white(package).bold()
            ),
            LogItemType::ExtraAlignmentToCR(input) => write!(
                f,
                "There are more &'s than should be in a aligned environment (table, etc.) near {}.",
                Paint::white(input).bold()
            ),
        }
    }
}

/// Trait for all rules. The task of a rule is specifying the regular expression which is used to
/// extract information from log file. Then it gets found captures from which the rule creates
/// corresponding log item. Optionally, the rule can specify custom retrieval of captures from log
/// file.
pub trait Rule<'a> {
    /// Returns regular expression for extraction of information from log file.
    fn get_regex(&self) -> Regex;

    /// Transforms captures to log item.
    fn process(&'a self, Captures<'a>) -> LogItem<'a>;

    /// Retrieves captures from log file. Custom mechanism can be implemented.
    fn captures(&'a self, pattern: Regex, log: &'a str) -> Vec<Captures<'a>> {
        pattern.captures_iter(log).collect()
    }
}

// RULES

pub struct UndefinedControlSequence;
pub struct TooManyEndingBraces;
pub struct NotInMathMode;
pub struct RunawayArgument;
pub struct RunawayArgument2;
pub struct UnderfullHBox;
pub struct OverfullHBox;
pub struct MissingPackage;
pub struct InvalidOption;
pub struct ExtraAlignmentToCR;

impl<'a> Rule<'a> for UndefinedControlSequence {
    fn get_regex(&self) -> Regex {
        PatternBuilder::new()
            .error(r"Undefined control sequence\.")
            .raw(r"(?:.+\n.+\n)?")
            .location()
            .line(r".*(\\[^{\s]+).*")
            .into()
    }

    fn process(&'a self, captures: Captures<'a>) -> LogItem<'a> {
        LogItem::new(
            LogItemType::UndefinedControlSequence(captures.get(2).unwrap().as_str()),
            Location::Line(usize::from_str(captures.get(1).unwrap().as_str()).unwrap()),
        )
    }
}

impl<'a> Rule<'a> for TooManyEndingBraces {
    fn get_regex(&self) -> Regex {
        PatternBuilder::new()
            .error(r"Too many }'s\.")
            .location_with_arg()
            .into()
    }

    fn process(&'a self, captures: Captures<'a>) -> LogItem<'a> {
        LogItem::new(
            LogItemType::TooManyEndingBraces(captures.get(2).unwrap().as_str()),
            Location::Line(usize::from_str(captures.get(1).unwrap().as_str()).unwrap()),
        )
    }
}

impl<'a> Rule<'a> for NotInMathMode {
    fn get_regex(&self) -> Regex {
        PatternBuilder::new()
            .error(r"Missing \$ inserted\.")
            .line("<inserted text> ")
            .line(r".+\$")
            .location_with_arg()
            .into()
    }

    fn process(&'a self, captures: Captures<'a>) -> LogItem<'a> {
        LogItem::new(
            LogItemType::NotInMathMode(captures.get(2).unwrap().as_str()),
            Location::Line(usize::from_str(captures.get(1).unwrap().as_str()).unwrap()),
        )
    }

    fn captures(&'a self, pattern: Regex, log: &'a str) -> Vec<Captures<'a>> {
        pattern.captures_iter(log).step_by(2).collect()
    }
}

impl<'a> Rule<'a> for RunawayArgument {
    fn get_regex(&self) -> Regex {
        PatternBuilder::new()
            .line(r"Runaway argument\?")
            .any_on_line()
            .error(r"Paragraph ended before (\S+) was complete\.")
            .any_on_line()
            .any_on_line()
            .location()
            .into()
    }

    fn process(&'a self, captures: Captures<'a>) -> LogItem<'a> {
        LogItem::new(
            LogItemType::RunawayArgument(captures.get(1).unwrap().as_str()),
            Location::Line(usize::from_str(captures.get(2).unwrap().as_str()).unwrap() - 1),
        )
    }
}

impl<'a> Rule<'a> for RunawayArgument2 {
    fn get_regex(&self) -> Regex {
        PatternBuilder::new()
            .line(r"Runaway argument\?")
            .any_on_line()
            .error(r"File ended while scanning use of (\S+)\.")
            .into()
    }

    fn process(&'a self, captures: Captures<'a>) -> LogItem<'a> {
        LogItem::new(
            LogItemType::RunawayArgument(captures.get(1).unwrap().as_str()),
            Location::End,
        )
    }
}

impl<'a> Rule<'a> for UnderfullHBox {
    fn get_regex(&self) -> Regex {
        PatternBuilder::new()
            .line(r"Underfull \\hbox \(badness (\d+)\) in paragraph at lines (\d+)--\d+")
            .line(r"(.*)")
            .into()
    }

    fn process(&'a self, captures: Captures<'a>) -> LogItem<'a> {
        LogItem::new(
            LogItemType::UnderfullHBox(
                captures.get(3).unwrap().as_str(),
                usize::from_str(captures.get(1).unwrap().as_str()).unwrap(),
            ),
            Location::Line(usize::from_str(captures.get(2).unwrap().as_str()).unwrap()),
        )
    }
}

impl<'a> Rule<'a> for OverfullHBox {
    fn get_regex(&self) -> Regex {
        PatternBuilder::new()
            .line(r"Overfull \\hbox \([^)]+\) in paragraph at lines (\d+)--\d+")
            .raw(r"\S+ ((?:.|\s)+)\[\]")
            .into()
    }

    fn process(&'a self, captures: Captures<'a>) -> LogItem<'a> {
        LogItem::new(
            LogItemType::OverfullHBox(captures.get(2).unwrap().as_str()),
            Location::Line(usize::from_str(captures.get(1).unwrap().as_str()).unwrap()),
        )
    }
}

impl<'a> Rule<'a> for MissingPackage {
    fn get_regex(&self) -> Regex {
        PatternBuilder::new()
            .error(r"LaTeX Error: File `([^\.]+)\.sty' not found\.")
            .into()
    }

    fn process(&'a self, captures: Captures<'a>) -> LogItem<'a> {
        LogItem::new(
            LogItemType::MissingPackage(captures.get(1).unwrap().as_str()),
            Location::None,
        )
    }
}

impl<'a> Rule<'a> for InvalidOption {
    fn get_regex(&self) -> Regex {
        PatternBuilder::new()
            .error(r"LaTeX Error: Unknown option `([^']+)' for package `([^']+)'\.")
            .into()
    }

    fn process(&'a self, captures: Captures<'a>) -> LogItem<'a> {
        LogItem::new(
            LogItemType::InvalidOption(
                captures.get(1).unwrap().as_str(),
                captures.get(2).unwrap().as_str(),
            ),
            Location::None,
        )
    }
}

impl<'a> Rule<'a> for ExtraAlignmentToCR {
    fn get_regex(&self) -> Regex {
        PatternBuilder::new()
            .error(r"Extra alignment tab has been changed to \\cr\.")
            .any_on_line()
            .any_on_line()
            .location_with_arg()
            .into()
    }

    fn process(&'a self, captures: Captures<'a>) -> LogItem<'a> {
        LogItem::new(
            LogItemType::ExtraAlignmentToCR(captures.get(2).unwrap().as_str().trim()),
            Location::Line(usize::from_str(captures.get(1).unwrap().as_str()).unwrap()),
        )
    }
}

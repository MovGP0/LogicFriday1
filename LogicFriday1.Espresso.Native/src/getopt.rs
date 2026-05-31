//! Minimal option scanner ported from Espresso's `getopt.c`.
//!
//! Espresso's command-line front end uses the classic C getopt state machine:
//! grouped one-letter options are scanned left to right, option arguments may
//! be attached to the option or placed in the next argv entry, and scanning
//! stops at `--` or the first non-option argument.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GetOpt {
    optind: usize,
    scan: Option<(usize, usize)>,
    optarg: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GetOptItem {
    Option(char),
    Unknown(char),
    End,
}

impl Default for GetOpt {
    fn default() -> Self {
        Self {
            optind: 0,
            scan: None,
            optarg: None,
        }
    }
}

impl GetOpt {
    pub fn optind(&self) -> usize {
        self.optind
    }

    pub fn optarg(&self) -> Option<&str> {
        self.optarg.as_deref()
    }

    pub fn next(&mut self, argv: &[String], optstring: &str) -> GetOptItem {
        self.optarg = None;

        let (arg_index, byte_index) = match self.scan {
            Some((arg_index, byte_index)) if byte_index < argv[arg_index].len() => {
                (arg_index, byte_index)
            }
            _ => {
                if self.optind == 0 {
                    self.optind = 1;
                }
                if self.optind >= argv.len() {
                    return GetOptItem::End;
                }
                let place = &argv[self.optind];
                if !place.starts_with('-') || place == "-" {
                    return GetOptItem::End;
                }
                self.optind += 1;
                if place == "--" {
                    return GetOptItem::End;
                }
                (self.optind - 1, 1)
            }
        };

        let arg = &argv[arg_index];
        let Some(c) = arg[byte_index..].chars().next() else {
            self.scan = None;
            return GetOptItem::End;
        };
        let next_index = byte_index + c.len_utf8();
        self.scan = (next_index < arg.len()).then_some((arg_index, next_index));

        let Some(opt_pos) = optstring.find(c) else {
            return GetOptItem::Unknown(c);
        };
        if c == ':' {
            return GetOptItem::Unknown(c);
        }

        if optstring[opt_pos + c.len_utf8()..].starts_with(':') {
            if next_index < arg.len() {
                self.optarg = Some(arg[next_index..].to_string());
                self.scan = None;
            } else if self.optind < argv.len() {
                self.optarg = Some(argv[self.optind].clone());
                self.optind += 1;
                self.scan = None;
            } else {
                self.optarg = None;
            }
        }

        GetOptItem::Option(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scans_grouped_options_and_attached_arguments_like_c_port() {
        let argv = ["espresso", "-abc", "-ofile.pla", "input.pla"]
            .into_iter()
            .map(String::from)
            .collect::<Vec<_>>();
        let mut getopt = GetOpt::default();

        assert_eq!(getopt.next(&argv, "abo:"), GetOptItem::Option('a'));
        assert_eq!(getopt.next(&argv, "abo:"), GetOptItem::Option('b'));
        assert_eq!(getopt.next(&argv, "abo:"), GetOptItem::Unknown('c'));
        assert_eq!(getopt.next(&argv, "abo:"), GetOptItem::Option('o'));
        assert_eq!(getopt.optarg(), Some("file.pla"));
        assert_eq!(getopt.next(&argv, "abo:"), GetOptItem::End);
        assert_eq!(getopt.optind(), 3);
    }

    #[test]
    fn consumes_separate_option_argument() {
        let argv = ["espresso", "-o", "out.pla"]
            .into_iter()
            .map(String::from)
            .collect::<Vec<_>>();
        let mut getopt = GetOpt::default();

        assert_eq!(getopt.next(&argv, "o:"), GetOptItem::Option('o'));
        assert_eq!(getopt.optarg(), Some("out.pla"));
        assert_eq!(getopt.optind(), 3);
    }

    #[test]
    fn stops_at_double_dash_or_first_non_option() {
        let argv = ["espresso", "--", "-x"]
            .into_iter()
            .map(String::from)
            .collect::<Vec<_>>();
        let mut getopt = GetOpt::default();
        assert_eq!(getopt.next(&argv, "x"), GetOptItem::End);
        assert_eq!(getopt.optind(), 2);

        let argv = ["espresso", "input.pla"]
            .into_iter()
            .map(String::from)
            .collect::<Vec<_>>();
        let mut getopt = GetOpt::default();
        assert_eq!(getopt.next(&argv, "x"), GetOptItem::End);
        assert_eq!(getopt.optind(), 1);
    }
}

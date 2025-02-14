use std::process;

use getopts::{Matches, Options};

pub struct CliParser {
    opts: Options,
}

impl CliParser {
    pub fn new() -> CliParser {
        let mut opts = Options::new();
        opts.long_only(true);
        opts.optopt(
            "",
            "exclude",
            r#"Explicitly exclude (comma-separated) tables from export and import
            "all" excludes all tables"#,
            "string",
        );
        opts.optopt(
            "", 
            "export",
            r#"Export Public-DB as SQL query. Possible values are "mysql" and "sqlite" (default "mysql")"#,
            "string");
        opts.optflag(
            "",
            "import",
            "Import configured SQL database to JSON database found in data folder",
        );
        opts.optflag("", "import-schema", "Import all schemas from database");
        opts.optopt(
            "", 
            "include",
            "Explicitly include (comma-separated) tables that are not listed or are non-static for import",
            "string");
        opts.optflag(
            "",
            "update-only",
            "Set to export/replace static content, but keep player content untouched",
        );
        opts.optflag("", "help", "Show this help");
        CliParser { opts }
    }

    pub fn get_matches(&self, args: &Vec<&str>) -> Matches {
        match self.opts.parse(args) {
            Ok(matches) => matches,
            Err(e) => {
                match e {
                    getopts::Fail::ArgumentMissing(o) => {
                        eprintln!("Option --{o} needs an argument.")
                    }
                    getopts::Fail::UnrecognizedOption(o) => {
                        eprintln!("Option --{o} is not defined.")
                    }
                    getopts::Fail::OptionMissing(o) => eprintln!("Option --{o} is missing."),
                    getopts::Fail::OptionDuplicated(o) => eprintln!("Option --{o} may not be given more than once."),
                    getopts::Fail::UnexpectedArgument(o) => eprintln!("Option --{o} may not be given an argument."),
                };
                eprintln!();
                eprint!("{}", self.usage_text(args.get(0).unwrap()));
                process::exit(2)
            }
        }
    }

    fn usage_text(&self, path_to_binary: &str) -> String {
        let brief = format!("Usage: {} [options]", path_to_binary);
        self.opts.usage(&brief)
    }

    pub fn print_usage(&self, path_to_binary: &str) {
        print!("{}", self.usage_text(path_to_binary));
    }
}

mod cli_parser;
pub mod yml_parser;

use core::panic;

use cli_parser::CliParser;
use yml_parser::{ConfigYml, DbCred};

use crate::db::{self, json_db::INTERNAL_DB, SqlProvider, TableSchema};

#[derive(Debug)]
pub struct Config {
    db_credentials: Option<DbCred>,
    pub db_provider: String,
    pub import_db: bool,
    pub import_schema: bool,
    exclude_tables: Vec<String>,
    include_tables: Vec<String>,
    ignored_tables: Vec<String>,
    update_only: bool,
}

impl Config {
    pub fn build(args: &Vec<&str>) -> Config {
        let cli_config = Config::parse_cli(args);
        let yml = ConfigYml::parse();

        let mut exclude_tables = cli_config.exclude_tables;
        exclude_tables.extend(yml.exclude);
        let mut include_tables = cli_config.include_tables;
        include_tables.extend(yml.include);
        let import_flag = cli_config.import_db;
        let import_schema_flag = cli_config.import_schema;
        let export_type = cli_config.db_provider;
        Config {
            db_provider: Config::get_db_type(
                &export_type,
                import_flag || import_schema_flag,
                &yml.db,
            ),
            db_credentials: Some(yml.db),
            import_db: import_flag,
            import_schema: import_schema_flag,
            exclude_tables,
            include_tables,
            ignored_tables: yml.exportignore,
            update_only: cli_config.update_only,
        }
    }

    fn get_selected_table_schemas(&self) -> Vec<&TableSchema> {
        let all_schemas = INTERNAL_DB.get_all_schemas();

        let included_table_names: Vec<_> = self
            .include_tables
            .iter()
            .map(|t| t.to_lowercase())
            .collect();
        let excluded_table_names: Vec<_> = self
            .exclude_tables
            .iter()
            .map(|t| t.to_lowercase())
            .chain(self.ignored_tables.iter().map(|t| t.to_lowercase()))
            .collect();

        if included_table_names.contains(&String::from("all")) {
            return all_schemas.iter().collect();
        }
        if excluded_table_names.contains(&String::from("all")) {
            return all_schemas
                .into_iter()
                .filter(|s| included_table_names.contains(&s.name.to_lowercase()))
                .collect::<Vec<_>>();
        }

        let ignored_when_importing = |s: &TableSchema| !s.is_static() && (self.import_db || self.update_only);
        return all_schemas
            .into_iter()
            .filter(|s| {
                !(excluded_table_names.contains(&s.name.to_lowercase())
                    || ignored_when_importing(s))
                    || included_table_names.contains(&s.name.to_lowercase())
            })
            .collect::<Vec<_>>();
    }

    pub fn get_excluded_schemas(&self) -> Vec<&TableSchema>{
        let selected_schemas: Vec<_> = self
            .get_selected_table_schemas()
            .iter()
            .map(|s| s.name.clone())
            .collect();
        let all_schemas = INTERNAL_DB.get_all_schemas();
        all_schemas.iter().filter(|e| !selected_schemas.contains(&e.name)).collect()
    }

    fn parse_cli(args: &Vec<&str>) -> Config {
        let cli_parser = CliParser::new();
        let matches = cli_parser.get_matches(args);
        if matches.opt_present("help") {
            cli_parser.print_usage(&args[0]);
            std::process::exit(0);
        }

        Config {
            db_credentials: None,
            db_provider: matches
                .opt_str("export")
                .unwrap_or_else(|| String::from("mysql")),
            import_db: matches.opt_present("import"),
            import_schema: matches.opt_present("import-schema"),
            exclude_tables: split_string(matches.opt_str("exclude").unwrap_or_default(), ","),
            include_tables: split_string(matches.opt_str("include").unwrap_or_default(), ","),
            ignored_tables: vec![],
            update_only: matches.opt_present("update-only"),
        }
    }

    fn get_db_type(export_type: &String, use_file_config: bool, db_cred: &DbCred) -> String {
        let import_sqlite = match db_cred {
            DbCred::Sqlite(_) => use_file_config,
            _ => false,
        };

        if export_type == "update-only" {
            println!(r#"Export type update-only is deprecated. Use "-update-only" instead."#);
        }
        if import_sqlite || export_type == "sqlite" {
            return String::from("sqlite");
        } else if use_file_config || export_type == "mysql" || export_type == "update-only" {
            return String::from("mysql");
        } else {
            panic!(
                r#"Chosen export value is invalid. Please choose either "mysql", "sqlite" or "update-only"."#
            )
        }
    }

    pub fn get_task(&self) -> Box<dyn Task + '_> {
        if self.import_schema {
            return Box::new(ImportSchema {
                db_credentials: self.db_credentials.clone().unwrap(),
            });
        }

        if self.import_db {
            return Box::new(ImportDb {
                db_credentials: self.db_credentials.clone().unwrap(),
                tables_to_import: self.get_selected_table_schemas(),
            });
        }

        return Box::new(ExportToSql {
            sql_provider: match self.db_provider.as_str() {
                "sqlite" => SqlProvider::Sqlite,
                "mysql" => SqlProvider::MySql,
                s => panic!(r#"Export to "{}" is not defined"#, s),
            },
            tables_to_export: match self.update_only {
                true => self.get_selected_table_schemas(),
                false => self.get_selected_table_schemas(),
            }
        });
    }
}

pub trait Task {
    fn execute(&self);
}

struct ImportSchema {
    db_credentials: DbCred,
}

impl Task for ImportSchema {
    fn execute(&self) {
        let schemas = get_db(&self.db_credentials).get_all_schemas();
        INTERNAL_DB.replace_schemas(schemas);
    }
}

struct ImportDb<'a> {
    db_credentials: DbCred,
    tables_to_import: Vec<&'a TableSchema>,
}

impl<'a> Task for ImportDb<'a> {
    fn execute(&self) {
        let db = get_db(&self.db_credentials);
        let table_count = self.tables_to_import.len();
        for (i, schema) in self.tables_to_import.iter().enumerate() {
            let texts = db.get_table_as_json(&schema.name);
            INTERNAL_DB.replace_table(texts, schema);
            println!("Finished {} of {} ({})", i + 1, table_count, schema.name);
        }
    }
}

struct ExportToSql<'a> {
    sql_provider: SqlProvider,
    tables_to_export: Vec<&'a TableSchema>,
}

impl<'a> Task for ExportToSql<'a> {
    fn execute(&self) {
        INTERNAL_DB.export_to_sql(&self.tables_to_export, &self.sql_provider)
    }
}

fn get_db(db_credentials: &DbCred) -> Box<dyn db::Db> {
    let db: Box<dyn db::Db> = match db_credentials {
        DbCred::Mysql(cred) => Box::new(db::mysql::Mysql::open(cred)),
        DbCred::Sqlite(cred) => Box::new(db::sqlite::Sqlite::open(cred)),
    };
    db
}

fn split_string(s: String, pattern: &str) -> Vec<String> {
    s.split(pattern)
        .filter(|s| !s.is_empty())
        .map(String::from)
        .collect::<Vec<_>>()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_build_no_args_given_get_defaults() {
        let config = Config::parse_cli(&vec!["path_to_binary"]);

        assert_eq!("mysql", config.db_provider);
        assert_eq!(false, config.import_db);
        assert_eq!(false, config.import_schema);
        assert_eq!(Vec::<String>::new(), config.ignored_tables);
        assert_eq!(Vec::<String>::new(), config.exclude_tables);
        assert_eq!(Vec::<String>::new(), config.include_tables);
        assert_eq!(false, config.update_only);
    }

    #[test]
    fn config_build_export_sqlite_db_provider_is_sqlite() {
        let args = vec!["path_to_binary", "--export", "sqlite"];

        let config = Config::parse_cli(&args);

        assert_eq!(config.db_provider, "sqlite")
    }

    #[test]
    fn config_build_exclude_abc_exclude_tables_has_vector_with_a_b_c() {
        let args = &vec!["path_to_binary", "--exclude", "a,b,c"];

        let config = Config::parse_cli(&args);

        assert_eq!(config.exclude_tables, vec!["a", "b", "c"])
    }
}

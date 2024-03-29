use crate::features::FeatureVersion;
use crate::lc::Passband;

#[cfg(feature = "hdf")]
use base64::{self, Engine};
use clap::{App, Arg, ArgMatches};
#[cfg(feature = "hdf")]
use md5;
#[cfg(feature = "hdf")]
use std::ops::Deref;
use std::path::Path;

pub fn arg_matches() -> ArgMatches<'static> {
    App::new("Query light curves and extrac features")
        .arg(
            Arg::with_name("database")
                .required(true)
                .possible_values(&["clickhouse"])
                .index(1)
                .help("Database (DB) type"),
        )
        .arg(Arg::with_name("sql_query").required(true).index(2).help(
            "SQL query to be sent to DB\
                Must return a response with these columns in this particular order:\
                sid, mjd, filter, mag, magerr",
        ))
        .arg(
            Arg::with_name("connection_config")
                .short("c")
                .long("connect")
                .takes_value(true)
                .default_value_ifs(&[("database", Some("clickhouse"), "tcp://localhost:9000")])
                .help("Connection configuration in form used by chosen DB"),
        )
        .arg(
            Arg::with_name("dir_output")
                .short("d")
                .long("dir")
                .takes_value(true)
                .default_value(".")
                .help("Directory path to output results"),
        )
        .arg(
            Arg::with_name("suffix")
                .short("s")
                .long("suffix")
                .takes_value(true)
                .default_value("")
                .help(
                    "Filename suffix, output filenames will be like <dir_output>/sid<suffix>.dat",
                ),
        )
        .arg(
            Arg::with_name("light_curves_are_sorted")
                .long("sorted")
                .takes_value(false)
                .help(
                    "Each input light curve is sorted by its 'mjd'.\
                    Note that this tool never groups light curves by sid, it must be done by DB",
                ),
        )
        .arg(
            Arg::with_name("passbands")
                .long("passbands")
                .takes_value(true)
                .default_value("gr")
                .help("Passbands to use"),
        )
        .arg(
            Arg::with_name("interpolate")
                .short("i")
                .long("interpol")
                .takes_value(false)
                .help("Do interpolation"),
        )
        .arg(
            Arg::with_name("features")
                .short("f")
                .long("features")
                .takes_value(false)
                .help("Do feature extraction"),
        )
        .arg(
            Arg::with_name("feature-version")
                .long("feature-version")
                .takes_value(true)
                .default_value("snad4")
                .help("Version of the feature extractor"),
        )
        .arg(
            Arg::with_name("cache_dir")
                .long("cache")
                .takes_value(true)
                .help(
                    "When specified, the DB response is cached to the given directory, \
                    use '-' to cache into <dir_output>",
                ),
        )
        .arg(
            Arg::with_name("no_sid")
                .long("no-sid")
                .takes_value(false)
                .help("Do not output sid data file"),
        )
        .get_matches()
}

pub enum DataBase {
    ClickHouse,
}

pub struct Config {
    pub database: DataBase,
    pub sql_query: String,
    pub connection_config: String,
    pub light_curves_are_sorted: bool,
    pub passbands: Vec<Passband>,
    pub sid_path: Option<String>,
    pub interpolation_config: Option<InterpolationConfig>,
    pub feature_config: Option<FeatureConfig>,
    pub cache_config: Option<CacheConfig>,
}

impl Config {
    fn get_path(root: &str, basename: &str, suffix: &str, ext: &str) -> String {
        let filename = format!("{}{}{}", basename, suffix, ext);
        let p = Path::new(root).join(filename);
        String::from(p.to_str().unwrap())
    }

    fn new(
        database_type: &str,
        sql_query: &str,
        connection_config: &str,
        output_dir: &str,
        suffix: &str,
        light_curves_are_sorted: bool,
        passbands_str: &str,
        interpolation_enabled: bool,
        features_enabled: bool,
        feature_version: &str,
        cache_dir: Option<&str>,
        no_sid: bool,
    ) -> Self {
        let database = match database_type {
            "clickhouse" => DataBase::ClickHouse,
            _ => panic!("only clickhouse database is supported"),
        };
        let passbands = passbands_str
            .chars()
            .map(|c| c.to_string().into())
            .collect();
        let sid_path = match !no_sid {
            true => Some(Self::get_path(output_dir, "sid", suffix, ".dat")),
            false => None,
        };
        let interpolation_config = if interpolation_enabled {
            Some(InterpolationConfig {
                path: Self::get_path(output_dir, "flux", suffix, ".dat"),
            })
        } else {
            None
        };
        let feature_config = if features_enabled {
            Some(FeatureConfig {
                value_path: Self::get_path(output_dir, "feature", suffix, ".dat"),
                name_path: Self::get_path(output_dir, "feature", suffix, ".name"),
                json_path: Self::get_path(output_dir, "feature", suffix, ".json"),
                version: feature_version.parse().unwrap(),
            })
        } else {
            None
        };

        #[cfg(feature = "hdf")]
        let cache_config = cache_dir.map(|dir| {
            let query_hash =
                base64::engine::general_purpose::STANDARD.encode(md5::compute(sql_query).deref());
            let suffix = "_".to_owned() + &query_hash[..8];
            let query_path = Self::get_path(dir, database_type, &suffix, ".sql");
            let data_path = Self::get_path(dir, database_type, &suffix, ".hdf5");
            CacheConfig {
                query_hash,
                query_path,
                data_path,
            }
        });
        #[cfg(not(feature = "hdf"))]
        let cache_config = cache_dir.map(|_| {
            panic!("the application is built without hdf support, caching cannot be used")
        });

        Self {
            database,
            sql_query: String::from(sql_query),
            connection_config: String::from(connection_config),
            light_curves_are_sorted,
            passbands,
            sid_path,
            interpolation_config,
            feature_config,
            cache_config,
        }
    }

    pub fn from_arg_matches(matches: &ArgMatches) -> Self {
        let database = matches.value_of("database").unwrap();
        let sql_query = matches.value_of("sql_query").unwrap();
        let connection_config = matches.value_of("connection_config").unwrap();
        let output_dir = matches.value_of("dir_output").unwrap();
        let suffix = matches.value_of("suffix").unwrap();
        let light_curves_are_sorted = matches.is_present("light_curves_are_sorted");
        let passbands = matches.value_of("passbands").unwrap();
        let interpolation_enabled = matches.is_present("interpolate");
        let features_enabled = matches.is_present("features");
        let feature_version = matches.value_of("feature-version").unwrap();
        let cache_dir = matches.value_of("cache_dir").map(|s| match s {
            "-" => output_dir,
            _ => s,
        });
        let no_sid = matches.is_present("no_sid");
        Self::new(
            database,
            sql_query,
            connection_config,
            output_dir,
            suffix,
            light_curves_are_sorted,
            passbands,
            interpolation_enabled,
            features_enabled,
            feature_version,
            cache_dir,
            no_sid,
        )
    }
}

pub struct InterpolationConfig {
    pub path: String,
}

pub struct FeatureConfig {
    pub value_path: String,
    pub name_path: String,
    pub json_path: String,
    pub version: FeatureVersion,
}

pub struct CacheConfig {
    pub query_hash: String,
    pub query_path: String,
    pub data_path: String,
}

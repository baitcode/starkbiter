#![warn(missing_docs)]
use std::{
    fs::{self, write, File},
    io::{self, BufRead, BufReader, ErrorKind},
    path::PathBuf,
    process::Command,
};

mod digest;
use self::digest::ArbiterConfig;
use cainome::rs::Abigen;
use inflector::Inflector;

/// Uses cainome from cartridge to generate bindings.
///
/// This function takes one directory as a source path and other as a destination path.
/// Searches source files for JSON files and generates Rust bindings for each one of them.
///
/// # Returns
///
/// * `Ok(())` if command successfully generates the bindings.
/// * `Err(std::io::Error)` if the command execution fails or if there's an
///   error in generating the bindings. This can also include if the `forge`
///   tool is not installed.

pub(crate) fn cainome_bind(src: &str, dest: &str, use_debug: &bool) -> std::io::Result<()> {
    let arbiter_config = ArbiterConfig::new().unwrap_or_default();
    // TODO: ???
    // let project_bidnings_output_path = arbiter_config.bindings_path;

    let derives = if *use_debug {
        vec!["Debug", "PartialEq"]
    } else {
        vec!["PartialEq"]
    };

    let contract_derives = if *use_debug {
        vec![
            "Debug",
            "Clone",
            "Serialize",
            "Deserialize",
            "Debug",
            // "DeserializeOwned",
        ]
    } else {
        vec![
            "Clone",
            "Serialize",
            "Deserialize",
            "Debug",
            // "DeserializeOwned",
        ]
    };

    let out_path = PathBuf::from(dest);

    if !out_path.exists() {
        fs::create_dir_all(&out_path).map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Failed to create output directory: {}", e),
            )
        })?;
    }

    if !out_path.is_dir() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("Output path {} is not a directory", out_path.display()),
        ));
    }

    let src_path_any = PathBuf::from(src);

    if src_path_any.is_dir() {
        let files = fs::read_dir(src_path_any.clone()).map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Failed to read directory: {}", e),
            )
        })?;

        for entry in files {
            let src_file_path = entry?.path();

            if !src_file_path.is_file() {
                continue;
            }
            if !src_file_path.extension().map_or(false, |ext| ext == "json") {
                continue;
            }

            // If the file is a JSON file, we will generate bindings for it
            let file_name = src_file_path.file_name().unwrap().to_str().unwrap();

            let contract_name = file_name.split(".").next().unwrap_or("").to_pascal_case();
            let contract_file_name = format!("{}.rs", contract_name.to_snake_case());
            // What if empty?

            println!("Generating bindings for contract: {}", contract_name);

            // let mut aliases = HashMap::new();
            // aliases.insert(String::from("my::type::Event"), String::from("MyTypeEvent"));

            let abigen = Abigen::new(contract_name.as_str(), src_file_path.to_str().unwrap())
                // .with_types_aliases(aliases)
                .with_derives(derives.iter().map(|d| d.to_string()).collect())
                .with_execution_version(cainome::rs::ExecutionVersion::V3)
                .with_contract_derives(contract_derives.iter().map(|d| d.to_string()).collect())
                .with_imports(vec!["serde::{Serialize, Deserialize}".to_string()]);

            abigen
                .generate()
                .expect("Fail to generate bindings")
                .write_to_file(out_path.join(contract_file_name).to_str().unwrap())
                .unwrap();
        }
    }

    Ok(())
}

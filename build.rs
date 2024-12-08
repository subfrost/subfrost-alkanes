use anyhow::Result;
use flate2::write::GzEncoder;
use flate2::Compression;
use hex;
use std::env;
use std::fs;
use std::io::prelude::*;
use std::path::Path;
use std::process::{Command, Stdio};

fn compress(binary: Vec<u8>) -> Result<Vec<u8>> {
    let mut writer = GzEncoder::new(Vec::<u8>::with_capacity(binary.len()), Compression::best());
    writer.write_all(&binary)?;
    Ok(writer.finish()?)
}

fn build_alkane(wasm_str: &str, features: Vec<&'static str>) -> Result<()> {
    if features.len() != 0 {
        let _ = Command::new("cargo")
            .env("CARGO_TARGET_DIR", wasm_str)
            .arg("build")
            .arg("--release")
            .arg("--features")
            .arg(features.join(","))
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .spawn()?
            .wait()?;
        Ok(())
    } else {
        Command::new("cargo")
            .env("CARGO_TARGET_DIR", wasm_str)
            .arg("build")
            .arg("--release")
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .spawn()?
            .wait()?;
        Ok(())
    }
}

fn main() {
    let env_var = env::var_os("OUT_DIR").unwrap();
    let base_dir = Path::new(&env_var)
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    let out_dir = base_dir.join("release");
    let wasm_dir = base_dir.parent().unwrap().join("alkanes");
    fs::create_dir_all(&wasm_dir).unwrap();
    let wasm_str = wasm_dir.to_str().unwrap();
    let write_dir = Path::new(&out_dir)
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("src")
        .join("tests");

    fs::create_dir_all(&write_dir.join("std")).unwrap();
    let crates_dir = out_dir
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("alkanes");
    std::env::set_current_dir(&crates_dir).unwrap();
    let mods = fs::read_dir(&crates_dir)
        .unwrap()
        .filter_map(|v| {
            let name = v.ok()?.file_name().into_string().ok()?;
            Some(name)
        })
        .collect::<Vec<String>>();
    let files = mods
        .clone()
        .into_iter()
        .filter_map(|name| {
            Some(name)
        })
        .collect::<Vec<String>>();
    files.into_iter()
        .map(|v| -> Result<String> {
            std::env::set_current_dir(&crates_dir.clone().join(v.clone()))?;
            build_alkane(wasm_str, vec![])?;
            std::env::set_current_dir(&crates_dir)?;
            let subbed = v.clone().replace("-", "_");
            eprintln!(
                "write: {}",
                write_dir
                    .join("std")
                    .join(subbed.clone() + "_build.rs")
                    .into_os_string()
                    .to_str()
                    .unwrap()
            );
            let f: Vec<u8> = fs::read(
                &Path::new(&wasm_str)
                    .join("wasm32-unknown-unknown")
                    .join("release")
                    .join(subbed.clone() + ".wasm"),
            )?;
            let compressed: Vec<u8> = compress(f.clone())?;
            fs::write(&Path::new(&wasm_str).join("wasm32-unknown-unknown").join("release").join(subbed.clone() + ".wasm.gz"), &compressed)?;
            let data: String = hex::encode(&f);
            fs::write(
                &write_dir.join("std").join(subbed.clone() + "_build.rs"),
                String::from("use hex_lit::hex;\n#[allow(long_running_const_eval)]\npub fn get_bytes() -> Vec<u8> { (&hex!(\"")
                    + data.as_str()
                    + "\")).to_vec() }",
            )?;
            eprintln!(
                "build: {}",
                write_dir
                    .join("std")
                    .join(subbed.clone() + "_build.rs")
                    .into_os_string()
                    .to_str()
                    .unwrap()
            );
            Ok(subbed)
        })
        .collect::<Result<Vec<String>>>()
        .unwrap();
    eprintln!(
        "write test builds to: {}",
        write_dir
            .join("std")
            .join("mod.rs")
            .into_os_string()
            .to_str()
            .unwrap()
    );
    fs::write(
        &write_dir.join("std").join("mod.rs"),
        mods.into_iter()
            .map(|v| v.replace("-", "_"))
            .fold(String::default(), |r, v| {
                r + "pub mod " + v.as_str() + "_build;\n"
            }),
    )
    .unwrap();
}

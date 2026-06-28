use std::env;
use std::process::ExitCode;

use mpb_knowledge::{
    build_runtime_bundle, compute_target_fingerprint, read_runtime_bundle, validate_source_dir,
};

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let mut args = env::args().skip(1);
    match args.next().as_deref() {
        Some("validate-source") => {
            let source_dir = args.next().ok_or("validate-source requires <source-dir>")?;
            validate_source_dir(source_dir).map_err(|error| error.to_string())?;
            println!("source pack is valid");
            Ok(())
        }
        Some("build-bundle") => {
            let source_dir = args.next().ok_or("build-bundle requires <source-dir>")?;
            let output_dir = args.next().ok_or("build-bundle requires <output-dir>")?;
            let bundle =
                build_runtime_bundle(&source_dir, &output_dir).map_err(|error| error.to_string())?;
            println!(
                "built bundle {} for {}",
                bundle.manifest.pack_id, bundle.manifest.exact_fingerprint
            );
            Ok(())
        }
        Some("inspect-bundle") => {
            let bundle_path = args.next().ok_or("inspect-bundle requires <knowledge-index.json>")?;
            let bundle = read_runtime_bundle(bundle_path).map_err(|error| error.to_string())?;
            println!(
                "{} {} entities={} evidence={}",
                bundle.manifest.pack_id,
                bundle.manifest.schema_version,
                bundle.indexes.entities_by_id.len(),
                bundle.indexes.evidence_by_id.len()
            );
            Ok(())
        }
        Some("fingerprint") => {
            let instance_path = args.next().ok_or("fingerprint requires <instance-path>")?;
            let builder_version = args.next().ok_or("fingerprint requires <builder-version>")?;
            let lab_version = args.next().ok_or("fingerprint requires <lab-version>")?;
            let schema_version = args.next().ok_or("fingerprint requires <schema-version>")?;
            let fingerprint = compute_target_fingerprint(
                instance_path,
                &builder_version,
                &lab_version,
                &schema_version,
            )
            .map_err(|error| error.to_string())?;
            println!(
                "{}",
                serde_json::to_string_pretty(&fingerprint).map_err(|error| error.to_string())?
            );
            Ok(())
        }
        _ => Err(
            "usage: mpb-knowledge <validate-source SOURCE|build-bundle SOURCE OUTPUT|inspect-bundle BUNDLE|fingerprint INSTANCE BUILDER LAB SCHEMA>"
                .to_string(),
        ),
    }
}

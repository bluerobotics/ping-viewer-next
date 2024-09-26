use vergen_gix::{BuildBuilder, CargoBuilder, DependencyKind, GixBuilder};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    generate_build_details()?;

    #[cfg(feature = "build-frontend")]
    build_web();

    Ok(())
}

fn generate_build_details() -> Result<(), Box<dyn std::error::Error>> {
    vergen_gix::Emitter::default()
        .add_instructions(&BuildBuilder::all_build()?)?
        .add_instructions(&GixBuilder::all_git()?)?
        .add_instructions(
            CargoBuilder::all_cargo()?.set_dep_kind_filter(Some(DependencyKind::Normal)),
        )?
        .emit()?;
    Ok(())
}

#[cfg(feature = "build-frontend")]
fn build_web() {
    use std::{path::Path, process::Command};

    // Note that as we are not watching all files, sometimes we'd need to force this build
    println!("cargo:rerun-if-changed=./ping-viewer-next-frontend/dist");

    let frontend_dir = Path::new("./ping-viewer-next-frontend/");
    frontend_dir
        .try_exists()
        .expect("Frontend directory does not exist");

    Command::new("bun")
        .args(["--version"])
        .status()
        .expect("Failed to build frontend, `bun` appears to be not installed.");

    Command::new("bun")
        .args(["install", "--frozen-lockfile"])
        .current_dir(frontend_dir)
        .status()
        .expect("Bun install failed!");

    Command::new("bun")
        .args(["run", "build"])
        .current_dir(frontend_dir)
        .status()
        .expect("Bun build failed!");
}

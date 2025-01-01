use std::{
    fs, io,
    path::{Path, PathBuf},
    process, str,
};

use annotate_snippets::Renderer;

mod lints;

fn workspace_toml() -> PathBuf {
    let output = process::Command::new(env!("CARGO"))
        .arg("locate-project")
        .arg("--workspace")
        .arg("--message-format=plain")
        .output()
        .expect("failed to run cargo")
        .stdout;
    PathBuf::from(std::str::from_utf8(&output).unwrap().trim())
}

fn process_source_file(renderer: &Renderer, path: &Path) -> io::Result<()> {
    let path_string = path.as_os_str().to_string_lossy().into_owned();
    let bytes = fs::read(path)?;
    let source = str::from_utf8(&bytes).map_err(io::Error::other)?;
    let ast = syn::parse_file(source).map_err(io::Error::other)?;

    lints::apply::<lints::abbreviated::Abbreviated>(
        renderer,
        &path_string,
        source,
        &ast,
    );

    Ok(())
}

fn main() -> io::Result<()> {
    env_logger::init_from_env("LOG");

    let global_context = cargo::GlobalContext::default().unwrap();
    let workspace =
        cargo::core::Workspace::new(&workspace_toml(), &global_context)
            .unwrap();

    let renderer = Renderer::styled();

    for package in workspace.members() {
        let source = cargo::sources::PathSource::new(
            package.root(),
            package.package_id().source_id(),
            workspace.gctx(),
        );

        for source_file in source.list_files(package).unwrap() {
            log::info!("found file: {}", source_file.to_string_lossy());
            if source_file
                .extension()
                .map(|extension| extension == "rs")
                .unwrap_or_default()
            {
                log::info!(
                    "processing file: {}",
                    source_file.to_string_lossy()
                );
                process_source_file(&renderer, &source_file)?;
            }
        }
    }

    Ok(())
}

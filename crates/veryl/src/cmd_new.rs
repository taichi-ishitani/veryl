use crate::OptNew;
use log::info;
use miette::{IntoDiagnostic, Result, bail};
use std::fs::{self, File};
use std::io::Write;
use veryl_metadata::{Git, Metadata};

pub struct CmdNew {
    opt: OptNew,
}

impl CmdNew {
    pub fn new(opt: OptNew) -> Self {
        Self { opt }
    }

    pub fn exec(&self) -> Result<bool> {
        if self.opt.path.exists() {
            bail!("path \"{}\" exists", self.opt.path.to_string_lossy());
        }

        if let Some(name) = self.opt.path.file_name() {
            let name = name.to_string_lossy();

            let toml = Metadata::create_default_toml(&name).into_diagnostic()?;
            let toml_path = self.opt.path.join("Veryl.toml");

            fs::create_dir_all(&self.opt.path).into_diagnostic()?;
            let mut file = File::create(toml_path).into_diagnostic()?;
            write!(file, "{toml}").into_diagnostic()?;
            file.flush().into_diagnostic()?;

            let src_path = self.opt.path.join("src");
            fs::create_dir_all(&src_path).into_diagnostic()?;

            if Git::exists() {
                let gitignore = Metadata::create_default_gitignore();
                let gitignore_path = self.opt.path.join(".gitignore");

                let mut file = File::create(&gitignore_path).into_diagnostic()?;
                write!(file, "{gitignore}").into_diagnostic()?;
                file.flush().into_diagnostic()?;

                Git::init(&self.opt.path)?;
            }

            info!("Created \"{name}\" project");
        } else {
            bail!("path \"{}\" is not valid", self.opt.path.to_string_lossy());
        }

        Ok(true)
    }
}

use crate::OptDump;
use log::info;
use miette::{IntoDiagnostic, Result, WrapErr};
use std::fs;
use veryl_analyzer::Analyzer;
use veryl_metadata::Metadata;
use veryl_parser::Parser;

pub struct CmdDump {
    opt: OptDump,
}

impl CmdDump {
    pub fn new(opt: OptDump) -> Self {
        Self { opt }
    }

    pub fn exec(&self, metadata: &mut Metadata) -> Result<bool> {
        let paths = metadata.paths(&self.opt.files, true, true)?;

        let mut contexts = Vec::new();

        for path in &paths {
            info!("Processing file ({})", path.src.to_string_lossy());

            let input = fs::read_to_string(&path.src)
                .into_diagnostic()
                .wrap_err("")?;
            let parser = Parser::parse(&input, &path.src)?;
            let analyzer = Analyzer::new(metadata);
            analyzer.analyze_pass1(&path.prj, &path.src, &parser.veryl);

            contexts.push((path, input, parser, analyzer));
        }

        Analyzer::analyze_post_pass1();

        for (path, _, parser, analyzer) in &contexts {
            analyzer.analyze_pass2(&path.prj, &path.src, &parser.veryl);
        }

        let info = Analyzer::analyze_post_pass2();

        for (path, _, parser, analyzer) in &contexts {
            analyzer.analyze_pass3(&path.prj, &path.src, &parser.veryl, &info);
        }

        if self.opt.symbol_table {
            println!("{}", veryl_analyzer::symbol_table::dump());
        }

        if self.opt.assign_list {
            println!("{}", veryl_analyzer::symbol_table::dump_assign_list());
        }

        if self.opt.namespace_table {
            println!("{}", veryl_analyzer::namespace_table::dump());
        }

        if self.opt.type_dag {
            println!("{}", veryl_analyzer::type_dag::dump());
        }

        if self.opt.attribute_table {
            println!("{}", veryl_analyzer::attribute_table::dump());
        }

        if self.opt.unsafe_table {
            println!("{}", veryl_analyzer::unsafe_table::dump());
        }

        Ok(true)
    }
}

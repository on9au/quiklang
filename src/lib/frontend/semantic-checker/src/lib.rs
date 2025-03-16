//! # Semantic Checker
//!
//! The semantic checker is responsible for verifying that the AST is semantically correct.
//!

use quiklang_common::{
    data_structs::ast::package_module::{Module, Package},
    CompilationReport,
};

pub struct SemanticAnalyzer<'a> {
    compilation_report: &'a mut CompilationReport,
    package: &'a mut Package,
}

impl<'a> SemanticAnalyzer<'a> {
    pub fn new(compilation_report: &'a mut CompilationReport, package: &'a mut Package) -> Self {
        Self {
            compilation_report,
            package,
        }
    }

    // /// Analyze the AST to ensure that it is semantically correct.
    // pub fn analyze(&mut self) {
    //     // Analyze library if it exists.
    //     if let Some(lib) = &self.package.library {
    //         self.analyze_module(lib);
    //     }

    //     // Analyze each bin module.
    //     for bin in &self.package.binaries {
    //         self.analyze_module(bin.module);
    //     }
    // }

    // /// Analyze a module.
    // fn analyze_module(&mut self, module: &Module) {
    //     // Resolve names in the AST.
    //     self.resolve_names();
    // }

    /// Resolve names in the AST.
    fn resolve_names(&mut self) {}
}

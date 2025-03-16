pub mod data_structs;
pub mod errors;

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use codespan_reporting::diagnostic::{Diagnostic, Label};
use codespan_reporting::files::SimpleFiles;
use codespan_reporting::term::{self, termcolor::StandardStream};
use data_structs::ast::package_metadata::PackageMetadata;
use errors::{CompilerError, CompilerWarning};

#[derive(Debug, Clone)]
pub struct File {
    pub id: usize,
    pub name: PathBuf,  // e.g., "src/main.quik"
    pub source: String, // The actual source code
}

/// Manages all files involved in the compilation process.
#[derive(Debug, Default, Clone)]
pub struct FileStore {
    pub root_dir: PathBuf,
    files: HashMap<usize, File>,
    next_id: usize,
    pub project_metadata: Option<PackageMetadata>,
}

impl FileStore {
    /// Creates a new, empty file store.
    pub fn new() -> Self {
        Self {
            root_dir: PathBuf::new(),
            files: HashMap::new(),
            next_id: 0,
            project_metadata: None,
        }
    }

    /// Adds a new file to the store and returns its unique ID.
    pub fn add_file(&mut self, name: PathBuf, source: String) -> usize {
        let id = self.next_id;
        self.next_id += 1;
        self.files.insert(id, File { id, name, source });
        id
    }

    /// Retrieves a file by its ID.
    pub fn get_file(&self, id: usize) -> Option<&File> {
        self.files.get(&id)
    }

    /// Retrieves a file by its name.
    pub fn get_file_by_name(&self, name: &Path) -> Option<&File> {
        self.files.values().find(|f| f.name == name)
    }

    /// Return iterator over all files in the store.
    pub fn files(&self) -> impl Iterator<Item = &File> {
        self.files.values()
    }

    /// Loads all files from a project directory, including processing Package.toml.
    pub fn add_project_files(&mut self, project_dir: &Path) -> Result<(), std::io::Error> {
        // Load and parse Package.toml
        let project_metadata = PackageMetadata::from_toml(project_dir)?;

        self.project_metadata = Some(project_metadata);

        // Define src_dir
        let src_dir = project_dir.join("src");

        // Add all files in the src directory
        self.add_quik_files_from_dir(&src_dir, &src_dir)?;

        // Set the root directory
        self.root_dir = project_dir.into();

        Ok(())
    }

    /// Recursively adds all .quik files in a directory to the file store.
    fn add_quik_files_from_dir(
        &mut self,
        src_dir: &Path,
        dir: &Path,
    ) -> Result<(), std::io::Error> {
        if dir.exists() && dir.is_dir() {
            for entry in fs::read_dir(dir)? {
                let entry = entry?;
                let path = entry.path();

                if path.is_dir() {
                    // Recurse into subdirectory
                    self.add_quik_files_from_dir(src_dir, &path)?;
                } else if let Some(extension) = path.extension() {
                    if extension == "quik" {
                        // Read and add the .quik file
                        let source = fs::read_to_string(&path)?;
                        // Get the relative path from the src directory
                        let relative_path = path.strip_prefix(src_dir).unwrap();
                        let name = src_dir.join(relative_path);
                        self.add_file(name, source);
                    }
                }
            }
        } else {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Directory not found: {:?}", dir),
            ));
        }

        Ok(())
    }
}

#[derive(Debug, Default)]
pub struct CompilationReport {
    pub errors: Vec<CompilerError>,
    pub warnings: Vec<CompilerWarning>,
    pub file_store: FileStore,
}

impl CompilationReport {
    pub fn new(file_store: FileStore) -> Self {
        Self {
            errors: Vec::new(),
            warnings: Vec::new(),
            file_store,
        }
    }

    pub fn add_error(&mut self, error: CompilerError) {
        self.errors.push(error);
    }

    pub fn add_warning(&mut self, warning: CompilerWarning) {
        self.warnings.push(warning);
    }

    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    /// Reports all accumulated errors and warnings with file names.
    pub fn report(&self) {
        let config = codespan_reporting::term::Config::default();
        let writer = StandardStream::stderr(term::termcolor::ColorChoice::Auto);

        let mut files = SimpleFiles::new(); // Placeholder
        let mut file_id_map = std::collections::HashMap::new();

        // Add all files to `SimpleFiles` and map their `file_id`s
        for file in self.file_store.files.values() {
            let id = files.add(file.name.display().to_string(), &file.source);
            file_id_map.insert(file.id, id);
        }

        // Report Errors
        if !self.errors.is_empty() {
            for error in &self.errors {
                let diagnostic = match error {
                    CompilerError::LexerError(e) => {
                        let mut diag = Diagnostic::error().with_message(e.to_string());

                        if let Some(span) = e.span() {
                            if file_id_map.contains_key(&span.file_id) {
                                diag = diag.with_labels(vec![
                                    Label::primary(span.file_id, span.start..span.end)
                                        .with_message("here"),
                                ]);
                            } else {
                                // If file_id is not found in the map, provide a generic label
                                diag = diag.with_notes(vec!["File ID not recognized.".to_string()]);
                            }
                        } else {
                            // No span available, provide a general note
                            diag = diag
                                .with_notes(vec!["No location information available.".to_string()]);
                        }

                        if !e.suggestions().is_empty() {
                            diag = diag.with_notes(e.suggestions());
                        }

                        diag
                    }
                    CompilerError::ParserError(e) => {
                        let mut diag = Diagnostic::error().with_message(e.to_string());

                        if let Some(span) = e.span() {
                            if file_id_map.contains_key(&span.file_id) {
                                diag = diag.with_labels(vec![
                                    Label::primary(span.file_id, span.start..span.end)
                                        .with_message("here"),
                                ]);
                            } else {
                                // If file_id is not found in the map, provide a generic label
                                diag = diag.with_notes(vec!["File ID not recognized.".to_string()]);
                            }
                        } else {
                            // No span available, provide a general note
                            diag = diag
                                .with_notes(vec!["No location information available.".to_string()]);
                        }

                        if !e.suggestions().is_empty() {
                            diag = diag.with_notes(e.suggestions());
                        }

                        diag
                    } // Handle other CompilerError variants if necessary
                };

                // Emit the diagnostic
                if let Err(e) = term::emit(&mut writer.lock(), &config, &files, &diagnostic) {
                    eprintln!("Failed to emit diagnostic: {}", e);
                }
            }
        }

        // Report Warnings
        if !self.warnings.is_empty() {
            #[allow(
                unreachable_code,
                unused_variables,
                clippy::match_single_binding,
                // reason = "Remove this line when adding warning categories"
            )]
            for warning in &self.warnings {
                let diagnostic = match warning {
                    // CompilerWarning::UnusedVariable {
                    //     name,
                    //     span,
                    //     suggestion: _,
                    // } => Diagnostic::warning()
                    //     .with_message(format!("Unused Variable: '{}'", name))
                    //     .with_labels(vec![Label::primary(
                    //         *file_id_map.get(&span.file_id).unwrap(),
                    //         (span.start, span.end),
                    //     )
                    //     .with_message("not used here")]),
                    // CompilerWarning::DeprecatedFeature {
                    //     feature,
                    //     span,
                    //     suggestion: _,
                    // } => Diagnostic::warning()
                    //     .with_message(format!("Deprecated Feature: '{}'", feature))
                    //     .with_labels(vec![Label::primary(
                    //         *file_id_map.get(&span.file_id).unwrap(),
                    //         (span.start, span.end),
                    //     )
                    //     .with_message("used here")]),
                    // // Handle other warning categories
                    _ => todo!(),
                };

                codespan_reporting::term::emit(&mut writer, &config, &files, &diagnostic).unwrap();
            }
        }
    }
}

// #[cfg(test)]
// mod tests {
//     #[test]
//     fn test_file_store() {
//         use super::*;

//         let dir = std::path::PathBuf::from("/home/djpro/quiklang/gitignored/exampel");

//         let src_dir = dir.join("src");

//         println!("dir: {:?}", dir);

//         let mut file_store = FileStore::default();

//         file_store.add_quik_files_from_dir(&src_dir, &dir).unwrap();

//         println!("file_store: {:#?}", file_store);
//     }
// }

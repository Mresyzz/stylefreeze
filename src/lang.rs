use std::path::Path;
use tree_sitter::Language;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SupportedLang {
    TypeScript,
    Tsx,
    JavaScript,
    Python,
    Rust,
    Go,
    Json,
    C,
    Cpp,
}

impl SupportedLang {
    pub fn from_path<P: AsRef<Path>>(path: P) -> Option<Self> {
        let ext = path.as_ref().extension()?.to_str()?.to_lowercase();
        match ext.as_str() {
            "ts" | "mts" | "cts" => Some(Self::TypeScript),
            "tsx" | "jsx" => Some(Self::Tsx),
            "js" | "mjs" | "cjs" => Some(Self::JavaScript),
            "py" | "pyi" => Some(Self::Python),
            "rs" => Some(Self::Rust),
            "go" => Some(Self::Go),
            "json" | "jsonc" => Some(Self::Json),
            "c" | "h" => Some(Self::C),
            "cpp" | "cc" | "cxx" | "hpp" | "hh" | "hxx" => Some(Self::Cpp),
            _ => None,
        }
    }

    pub fn to_tree_sitter_language(self) -> Language {
        match self {
            Self::TypeScript | Self::JavaScript => {
                tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into()
            }
            Self::Tsx => tree_sitter_typescript::LANGUAGE_TSX.into(),
            Self::Python => tree_sitter_python::LANGUAGE.into(),
            Self::Rust => tree_sitter_rust::LANGUAGE.into(),
            Self::Go => tree_sitter_go::LANGUAGE.into(),
            Self::Json => tree_sitter_json::LANGUAGE.into(),
            Self::C => tree_sitter_c::LANGUAGE.into(),
            Self::Cpp => tree_sitter_cpp::LANGUAGE.into(),
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::TypeScript => "TypeScript",
            Self::Tsx => "TSX",
            Self::JavaScript => "JavaScript",
            Self::Python => "Python",
            Self::Rust => "Rust",
            Self::Go => "Go",
            Self::Json => "JSON",
            Self::C => "C",
            Self::Cpp => "C++",
        }
    }
}

use serde::ser::SerializeMap;
use serde::{Serialize, Serializer};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ParsedProgram {
    pub source: String,
    pub commands: Vec<SimpleCommand>,
    pub operators: Vec<OperatorOccurrence>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SimpleCommand {
    pub text: String,
    pub argv: Vec<String>,
    pub env: Vec<EnvAssignment>,
    pub redirects: Vec<Redirect>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct EnvAssignment {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub struct Span {
    pub start_byte: usize,
    pub end_byte: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperatorOccurrence {
    pub op: ShellOperator,
    pub span: Span,
}

impl Serialize for OperatorOccurrence {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.op.serialize(serializer)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum ShellOperator {
    #[serde(rename = "&&")]
    And,
    #[serde(rename = "||")]
    Or,
    #[serde(rename = "|")]
    Pipe,
    #[serde(rename = "|&")]
    PipeBoth,
    #[serde(rename = ";")]
    Sequence,
    #[serde(rename = "&")]
    Background,
    #[serde(rename = "\n")]
    Newline,
}

impl ShellOperator {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::And => "&&",
            Self::Or => "||",
            Self::Pipe => "|",
            Self::PipeBoth => "|&",
            Self::Sequence => ";",
            Self::Background => "&",
            Self::Newline => "\n",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Redirect {
    pub op: RedirectOp,
    pub target: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fd: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum RedirectOp {
    #[serde(rename = ">")]
    Write,
    #[serde(rename = ">>")]
    Append,
    #[serde(rename = "<")]
    Read,
    #[serde(rename = "<<")]
    HereDoc,
    #[serde(rename = ">&")]
    DuplicateOutput,
    #[serde(rename = ">|")]
    Clobber,
    #[serde(rename = "<&")]
    DuplicateInput,
    #[serde(rename = "&>")]
    WriteBoth,
    #[serde(rename = "&>>")]
    AppendBoth,
    #[serde(rename = "<<<")]
    HereString,
}

impl RedirectOp {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Write => ">",
            Self::Append => ">>",
            Self::Read => "<",
            Self::HereDoc => "<<",
            Self::DuplicateOutput => ">&",
            Self::Clobber => ">|",
            Self::DuplicateInput => "<&",
            Self::WriteBoth => "&>",
            Self::AppendBoth => "&>>",
            Self::HereString => "<<<",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TooComplexReason {
    UnsupportedNode,
    ErrorNode,
    LimitExceeded,
    SuspiciousInput,
    ParserAborted,
    DynamicExpansion,
    InvalidStructure,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TooComplex {
    pub reason: TooComplexReason,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub node_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

impl TooComplex {
    #[must_use]
    pub fn unsupported(node_type: impl Into<String>) -> Self {
        Self {
            reason: TooComplexReason::UnsupportedNode,
            node_type: Some(node_type.into()),
            detail: None,
        }
    }

    #[must_use]
    pub fn unsupported_with_detail(
        node_type: impl Into<String>,
        detail: impl Into<String>,
    ) -> Self {
        Self {
            reason: TooComplexReason::UnsupportedNode,
            node_type: Some(node_type.into()),
            detail: Some(detail.into()),
        }
    }

    #[must_use]
    pub fn dynamic(node_type: impl Into<String>, detail: impl Into<String>) -> Self {
        Self {
            reason: TooComplexReason::DynamicExpansion,
            node_type: Some(node_type.into()),
            detail: Some(detail.into()),
        }
    }

    #[must_use]
    pub fn invalid_structure(detail: impl Into<String>) -> Self {
        Self {
            reason: TooComplexReason::InvalidStructure,
            node_type: None,
            detail: Some(detail.into()),
        }
    }

    #[must_use]
    pub fn limit(node_type: impl Into<String>, detail: impl Into<String>) -> Self {
        Self {
            reason: TooComplexReason::LimitExceeded,
            node_type: Some(node_type.into()),
            detail: Some(detail.into()),
        }
    }

    #[must_use]
    pub fn suspicious(detail: impl Into<String>) -> Self {
        Self {
            reason: TooComplexReason::SuspiciousInput,
            node_type: None,
            detail: Some(detail.into()),
        }
    }

    #[must_use]
    pub fn parser_aborted(detail: impl Into<String>) -> Self {
        Self {
            reason: TooComplexReason::ParserAborted,
            node_type: Some("PARSE_ABORT".to_owned()),
            detail: Some(detail.into()),
        }
    }

    #[must_use]
    pub fn error_node(node_type: impl Into<String>) -> Self {
        Self {
            reason: TooComplexReason::ErrorNode,
            node_type: Some(node_type.into()),
            detail: Some("tree-sitter produced an ERROR or missing node".to_owned()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ParseError {
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GateResult {
    Parsed(ParsedProgram),
    TooComplex(TooComplex),
    ParseError(ParseError),
}

impl GateResult {
    #[must_use]
    pub const fn kind(&self) -> &'static str {
        match self {
            Self::Parsed(_) => "parsed",
            Self::TooComplex(_) => "too_complex",
            Self::ParseError(_) => "parse_error",
        }
    }
}

impl Serialize for GateResult {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut map = serializer.serialize_map(None)?;
        map.serialize_entry("kind", self.kind())?;
        match self {
            Self::Parsed(program) => {
                map.serialize_entry("source", &program.source)?;
                map.serialize_entry("commands", &program.commands)?;
                map.serialize_entry("operators", &program.operators)?;
            }
            Self::TooComplex(error) => {
                map.serialize_entry("reason", &error.reason)?;
                if let Some(node_type) = &error.node_type {
                    map.serialize_entry("node_type", node_type)?;
                }
                if let Some(detail) = &error.detail {
                    map.serialize_entry("detail", detail)?;
                }
            }
            Self::ParseError(error) => {
                map.serialize_entry("message", &error.message)?;
            }
        }
        map.end()
    }
}

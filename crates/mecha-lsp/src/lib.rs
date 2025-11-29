use dashmap::DashMap;
use mecha_compiler::parser::parse;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer};

pub struct Backend {
    client: Client,
    document_map: DashMap<String, String>,
}

impl Backend {
    pub fn new(client: Client) -> Self {
        Self {
            client,
            document_map: DashMap::new(),
        }
    }

    async fn on_change(&self, uri: Url, text: String) {
        self.document_map.insert(uri.to_string(), text.clone());

        let mut diagnostics = Vec::new();
        let schema_result = parse("", &text); // todo: add source name here

        match schema_result {
            Ok(mut schema) => {
                if let Err(errs) = schema.check() {
                    for err in errs {
                        let span = err.span();
                        let (start_line, start_col) = byte_index_to_line_col(&text, span.start);
                        let (end_line, end_col) = byte_index_to_line_col(&text, span.end);

                        diagnostics.push(Diagnostic {
                            range: Range {
                                start: Position {
                                    line: start_line,
                                    character: start_col,
                                },
                                end: Position {
                                    line: end_line,
                                    character: end_col,
                                },
                            },
                            severity: Some(DiagnosticSeverity::ERROR),
                            message: err.to_string(),
                            source: Some("mecha-lsp".to_string()),
                            ..Default::default()
                        });
                    }
                }
            }
            Err(errs) => {
                for err in errs {
                    let span = err.span();
                    let (start_line, start_col) = byte_index_to_line_col(&text, span.start);
                    let (end_line, end_col) = byte_index_to_line_col(&text, span.end);

                    diagnostics.push(Diagnostic {
                        range: Range {
                            start: Position {
                                line: start_line,
                                character: start_col,
                            },
                            end: Position {
                                line: end_line,
                                character: end_col,
                            },
                        },
                        severity: Some(DiagnosticSeverity::ERROR),
                        message: err.to_string(),
                        source: Some("mecha-lsp".to_string()),
                        ..Default::default()
                    });
                }
            }
        }

        self.client
            .publish_diagnostics(uri, diagnostics, None)
            .await;
    }
}

fn byte_index_to_line_col(text: &str, index: usize) -> (u32, u32) {
    let mut line = 0;
    let mut col = 0;
    for (i, c) in text.char_indices() {
        if i >= index {
            break;
        }
        if c == '\n' {
            line += 1;
            col = 0;
        } else {
            col += 1;
        }
    }
    (line, col)
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                semantic_tokens_provider: Some(
                    SemanticTokensServerCapabilities::SemanticTokensOptions(
                        SemanticTokensOptions {
                            legend: SemanticTokensLegend {
                                token_types: vec![
                                    SemanticTokenType::CLASS,
                                    SemanticTokenType::PROPERTY,
                                    SemanticTokenType::TYPE,
                                ],
                                token_modifiers: vec![],
                            },
                            full: Some(SemanticTokensFullOptions::Bool(true)),
                            ..Default::default()
                        },
                    ),
                ),
                ..Default::default()
            },
            ..Default::default()
        })
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        self.on_change(params.text_document.uri, params.text_document.text)
            .await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        if let Some(change) = params.content_changes.into_iter().next() {
            self.on_change(params.text_document.uri, change.text).await;
        }
    }

    async fn semantic_tokens_full(
        &self,
        params: SemanticTokensParams,
    ) -> Result<Option<SemanticTokensResult>> {
        let uri = params.text_document.uri.to_string();
        let Some(text) = self.document_map.get(&uri) else {
            return Ok(None);
        };

        let Ok(schema) = parse("", &text) else { // todo: add source name here
            return Ok(None);
        };

        let mut semantic_tokens = Vec::new();

        let mut raw_tokens: Vec<(u32, u32, u32, u32)> = Vec::new();

        for table in schema.tables {
            let span = table.id.span;
            let (line, col) = byte_index_to_line_col(&text, span.start);
            let len = (span.end - span.start) as u32;
            raw_tokens.push((line, col, len, 0)); // 0 = CLASS

            for column in table.columns {
                let span = column.id.span;
                let (line, col) = byte_index_to_line_col(&text, span.start);
                let len = (span.end - span.start) as u32;
                raw_tokens.push((line, col, len, 1)); // 1 = PROPERTY

                let span = column.typ.span;
                let (line, col) = byte_index_to_line_col(&text, span.start);
                let len = (span.end - span.start) as u32;
                raw_tokens.push((line, col, len, 2)); // 2 = TYPE
            }
        }

        raw_tokens.sort_by(|a, b| {
            if a.0 != b.0 {
                a.0.cmp(&b.0)
            } else {
                a.1.cmp(&b.1)
            }
        });

        let mut pre_line = 0;
        let mut pre_start = 0;

        for (line, col, len, token_type) in raw_tokens {
            let delta_line = line - pre_line;
            let delta_start = if delta_line == 0 {
                col - pre_start
            } else {
                col
            };

            semantic_tokens.push(SemanticToken {
                delta_line,
                delta_start,
                length: len,
                token_type,
                token_modifiers_bitset: 0,
            });

            pre_line = line;
            pre_start = col;
        }

        Ok(Some(SemanticTokensResult::Tokens(SemanticTokens {
            result_id: None,
            data: semantic_tokens,
        })))
    }
}

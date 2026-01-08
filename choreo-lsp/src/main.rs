use choreo::parser::{linter, parser};
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::{
    Diagnostic, DiagnosticSeverity, DidChangeTextDocumentParams, DidOpenTextDocumentParams,
    InitializeParams, InitializeResult, InitializedParams, MessageType, NumberOrString, Position,
    Range, ServerCapabilities, TextDocumentItem, TextDocumentSyncCapability, TextDocumentSyncKind,
};
use tower_lsp::{Client, LanguageServer, LspService, Server};

#[derive(Debug)]
struct Backend {
    client: Client,
}

impl Backend {
    async fn on_change(&self, params: TextDocumentItem) {
        let uri = params.uri.clone();
        let diagnostics = match parser::parse(&params.text) {
            Ok(test_suite) => {
                let lint_diagnostics = linter::lint(&test_suite);

                lint_diagnostics
                    .into_iter()
                    .map(|d| {
                        let severity = match d.severity {
                            linter::Severity::Error => DiagnosticSeverity::ERROR,
                            linter::Severity::Warning => DiagnosticSeverity::WARNING,
                            linter::Severity::Info => DiagnosticSeverity::INFORMATION,
                        };

                        // Use the line number from the diagnostic, default to 0 if not available
                        let line = d.line.saturating_sub(1) as u32; // Convert 1-based to 0-based
                        let column = d.column.unwrap_or(0) as u32;

                        Diagnostic::new(
                            Range::new(
                                Position::new(line, column),
                                Position::new(line, column + 10), // Adjust range as needed
                            ),
                            Some(severity),
                            Some(NumberOrString::String(d.rule.code.to_string())),
                            Some("choreo-lsp".to_string()),
                            d.message,
                            None,
                            None,
                        )
                    })
                    .collect::<Vec<Diagnostic>>()
            }
            Err(e) => {
                vec![Diagnostic::new_simple(
                    Range::new(Position::new(0, 0), Position::new(0, 1)),
                    format!("Parsing error: {}", e),
                )]
            }
        };

        self.client
            .publish_diagnostics(uri, diagnostics, Some(params.version))
            .await;
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                ..ServerCapabilities::default()
            },
            ..InitializeResult::default()
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "choreo-lsp: server initialized!")
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        self.client
            .log_message(
                MessageType::INFO,
                format!("File opened: {}", params.text_document.uri),
            )
            .await;
        self.on_change(params.text_document).await;
    }

    async fn did_change(&self, mut params: DidChangeTextDocumentParams) {
        self.client
            .log_message(
                MessageType::INFO,
                format!("File changed: {}", params.text_document.uri),
            )
            .await;
        self.on_change(TextDocumentItem {
            uri: params.text_document.uri,
            text: params.content_changes.remove(0).text,
            version: params.text_document.version,
            language_id: "choreo".to_string(), // Or get from somewhere else
        })
        .await;
    }
}

#[tokio::main]
async fn main() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(|client| Backend { client });
    Server::new(stdin, stdout, socket).serve(service).await;
}

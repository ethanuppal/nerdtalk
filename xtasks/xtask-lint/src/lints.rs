use std::{cell::RefCell, rc::Rc};

use annotate_snippets::{Level, Renderer, Snippet};

pub mod abbreviated;

#[derive(Default)]
pub struct EmittedStatus {
    pub(crate) warned: bool,
    pub(crate) errored: bool,
}

pub struct DiagnosticContext<'a> {
    emitted_status: Rc<RefCell<EmittedStatus>>,
    renderer: &'a Renderer,
    path: &'a str,
    source: &'a str,
}

impl DiagnosticContext<'_> {
    pub fn new_snippet(&self) -> Snippet {
        Snippet::source(self.source).origin(self.path)
    }

    pub fn new_warning(&self, title: &str, snippet: Snippet) {
        let message = Level::Warning.title(title).snippet(snippet);
        println!("{}", self.renderer.render(message));
        self.emitted_status.borrow_mut().warned = true;
    }
}

pub trait WithDiagnosticContext<'a> {
    fn with_diagnostic_context(
        diagnostic_context: DiagnosticContext<'a>,
    ) -> Self;
}

pub fn apply<
    'a,
    L: WithDiagnosticContext<'a> + for<'ast> syn::visit::Visit<'ast>,
>(
    emitted_status: Rc<RefCell<EmittedStatus>>,
    renderer: &'a Renderer,
    path: &'a str,
    source: &'a str,
    ast: &syn::File,
) {
    L::with_diagnostic_context(DiagnosticContext {
        emitted_status,
        renderer,
        path,
        source,
    })
    .visit_file(ast);
}

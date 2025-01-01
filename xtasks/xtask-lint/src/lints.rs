use std::{cell::RefCell, rc::Rc};

use annotate_snippets::{Message, Renderer, Snippet};

pub mod abbreviated;

pub struct DiagnosticContext<'a> {
    emitted: Rc<RefCell<bool>>,
    renderer: &'a Renderer,
    path: &'a str,
    source: &'a str,
}

impl DiagnosticContext<'_> {
    fn new_snippet(&self) -> Snippet {
        Snippet::source(self.source).origin(self.path)
    }

    fn print(&self, message: Message) {
        *self.emitted.borrow_mut() = true;
        println!("{}", self.renderer.render(message));
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
    emitted: Rc<RefCell<bool>>,
    renderer: &'a Renderer,
    path: &'a str,
    source: &'a str,
    ast: &syn::File,
) {
    L::with_diagnostic_context(DiagnosticContext {
        emitted,
        renderer,
        path,
        source,
    })
    .visit_file(ast);
}

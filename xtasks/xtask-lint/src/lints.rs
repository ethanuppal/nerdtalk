use annotate_snippets::Renderer;

pub mod abbreviated;

pub trait WithRenderer<'a> {
    fn with_renderer(
        renderer: &'a Renderer,
        path: &'a str,
        source: &'a str,
    ) -> Self;
}

pub fn apply<'a, L: WithRenderer<'a> + for<'ast> syn::visit::Visit<'ast>>(
    renderer: &'a Renderer,
    path: &'a str,
    source: &'a str,
    ast: &syn::File,
) {
    L::with_renderer(renderer, path, source).visit_file(ast);
}

use std::{collections::HashMap, sync};

use annotate_snippets::{Level, Renderer, Snippet};
use proc_macro2::Span;
use syn::spanned::Spanned;

use super::WithRenderer;

pub struct Abbreviated<'a> {
    renderer: &'a Renderer,
    path: &'a str,
    source: &'a str,
}

static ABBREVIATIONS: sync::LazyLock<HashMap<&str, Vec<&str>>> =
    sync::LazyLock::new(|| {
        HashMap::from([
            ("msg", vec!["message"]),
            ("cmd", vec!["command"]),
            ("beg", vec!["begin", "beginning"]),
        ])
    });

#[derive(Clone, Copy)]
enum Case {
    /// for identifiers and function names
    Snake,
    /// for types
    Pascal,
}

fn split_by_case(str: &str, case: Case) -> Vec<(usize, String)> {
    let mut result = vec![];
    let mut last_split = 0;
    match case {
        Case::Snake => {
            for (i, c) in str.char_indices() {
                if c == '_' {
                    result.push((last_split, str[last_split..i].to_string()));
                    last_split = i + 1; // skip past underscore
                }
            }
            if last_split < str.len() {
                result.push((last_split, str[last_split..].to_string()));
            }
        }
        Case::Pascal => {
            for (i, c) in str.char_indices() {
                if i > 0 && c.is_ascii_uppercase() {
                    result
                        .push((last_split, str[last_split..i].to_lowercase()));
                    last_split = i;
                }
            }
            if last_split < str.len() {
                result.push((last_split, str[last_split..].to_lowercase()));
            }
        }
    }
    result
}

impl Abbreviated<'_> {
    fn check_words(&mut self, span: Span, ident: &str, case: Case) {
        for (relative_start, word) in split_by_case(ident, case) {
            if let Some(replacements) = ABBREVIATIONS.get(word.as_str()) {
                assert!(!replacements.is_empty());

                let absolute_start = span.byte_range().start + relative_start;
                let span_range = absolute_start..absolute_start + word.len();

                let replacements_cased = replacements
                    .iter()
                    .map(|replacement| {
                        let mut result = format!("`{}`", replacement);
                        if matches!(case, Case::Pascal) {
                            if let Some(first) = result.get_mut(0..1) {
                                first.make_ascii_uppercase();
                            }
                        }
                        result
                    })
                    .collect::<Vec<_>>();

                let replacements_string = match replacements_cased.len() {
                    1 => replacements_cased[0].to_string(),
                    2 => format!(
                        "{} or {}",
                        replacements_cased[0], replacements_cased[1]
                    ),
                    _ => {
                        let (last, head) =
                            replacements_cased.split_last().unwrap();
                        format!("{} or {}", head.join(", "), last)
                    }
                };

                println!(
                    "{}",
                    self.renderer.render(
                        Level::Warning.title("potential abbreviation").snippet(
                            Snippet::source(self.source)
                                .line_start(span.start().line)
                                .origin(self.path)
                                .fold(true)
                                .annotation(
                                    Level::Help.span(span_range).label(
                                        &format!(
                                            "Consider using {} instead",
                                            replacements_string
                                        ),
                                    )
                                ),
                        ),
                    )
                );
            }
        }
    }
}

impl<'ast> syn::visit::Visit<'ast> for Abbreviated<'_> {
    fn visit_item_fn(&mut self, function: &'ast syn::ItemFn) {
        let function_name = function.sig.ident.to_string();
        self.check_words(function.span(), &function_name, Case::Snake);
        syn::visit::visit_item_fn(self, function);
    }
    fn visit_type_path(&mut self, type_path: &'ast syn::TypePath) {
        let type_name = type_path
            .path
            .segments
            .last()
            .expect("empty path")
            .ident
            .to_string();
        self.check_words(type_path.span(), &type_name, Case::Pascal);
    }

    fn visit_path(&mut self, path: &'ast syn::Path) {
        let ident = path.segments.last().expect("empty path").ident.to_string();
        self.check_words(path.span(), &ident, Case::Snake);
    }

    fn visit_ident(&mut self, ident: &'ast proc_macro2::Ident) {
        let ident_string = ident.to_string();
        self.check_words(ident.span(), &ident_string, Case::Snake);
    }
}

impl<'a> WithRenderer<'a> for Abbreviated<'a> {
    fn with_renderer(
        renderer: &'a Renderer,
        path: &'a str,
        source: &'a str,
    ) -> Self {
        Self {
            renderer,
            path,
            source,
        }
    }
}

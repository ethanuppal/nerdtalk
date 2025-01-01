use std::{collections::HashMap, sync};

use annotate_snippets::Level;

use super::{DiagnosticContext, WithDiagnosticContext};

pub struct Abbreviated<'a> {
    context: DiagnosticContext<'a>,
}

static ABBREVIATIONS: sync::LazyLock<HashMap<&str, Vec<&str>>> =
    sync::LazyLock::new(|| {
        HashMap::from([
            ("msg", vec!["message"]),
            ("cmd", vec!["command"]),
            ("beg", vec!["begin", "beginning"]),
            ("jeff", vec!["rust_enjoyer"]),
            ("len", vec!["length"]),
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
    fn check_words(&mut self, ident: &syn::Ident, case: Case) {
        let span = ident.span();
        for (relative_start, word) in split_by_case(&ident.to_string(), case) {
            if let Some(replacements) = ABBREVIATIONS.get(word.as_str()) {
                assert!(!replacements.is_empty());

                let absolute_start = span.byte_range().start + relative_start;
                let span_range = absolute_start..absolute_start + word.len();

                let replacements_cased = replacements
                    .iter()
                    .map(|replacement| {
                        let mut result = format!("`{}`", replacement);
                        if matches!(case, Case::Pascal) {
                            if let Some(first) = result.get_mut(1..2) {
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

                self.context.new_warning(
                    "potential abbreviation",
                    self.context
                        .new_snippet()
                        .line_start(span.start().line)
                        .fold(true)
                        .annotation(Level::Help.span(span_range).label(
                            &format!(
                                "Consider using {} instead",
                                replacements_string
                            ),
                        )),
                );
            }
        }
    }
}

// check all user-defined name sites

impl<'ast> syn::visit::Visit<'ast> for Abbreviated<'_> {
    fn visit_item_fn(&mut self, function: &'ast syn::ItemFn) {
        self.check_words(&function.sig.ident, Case::Snake);
        syn::visit::visit_item_fn(self, function);
    }

    fn visit_item_struct(&mut self, item_struct: &'ast syn::ItemStruct) {
        self.check_words(&item_struct.ident, Case::Pascal);
        syn::visit::visit_item_struct(self, item_struct);
    }

    fn visit_item_enum(&mut self, item_enum: &'ast syn::ItemEnum) {
        self.check_words(&item_enum.ident, Case::Pascal);
        syn::visit::visit_item_enum(self, item_enum);
    }

    fn visit_item_type(&mut self, item_type: &'ast syn::ItemType) {
        self.check_words(&item_type.ident, Case::Pascal);
        syn::visit::visit_item_type(self, item_type);
    }

    fn visit_item_const(&mut self, item_const: &'ast syn::ItemConst) {
        self.check_words(&item_const.ident, Case::Snake);
        syn::visit::visit_item_const(self, item_const);
    }

    fn visit_item_static(&mut self, item_static: &'ast syn::ItemStatic) {
        self.check_words(&item_static.ident, Case::Snake);
        syn::visit::visit_item_static(self, item_static);
    }

    fn visit_pat_ident(&mut self, ident_pattern: &'ast syn::PatIdent) {
        self.check_words(&ident_pattern.ident, Case::Snake);
        syn::visit::visit_pat_ident(self, ident_pattern);
    }
}

impl<'a> WithDiagnosticContext<'a> for Abbreviated<'a> {
    fn with_diagnostic_context(
        diagnostic_context: DiagnosticContext<'a>,
    ) -> Self {
        Self {
            context: diagnostic_context,
        }
    }
}

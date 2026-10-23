//! Bilingual (PT/EN) strings backed by Fluent. The `.ftl` files live in `/locales` and are
//! compiled into the binary, so the MCP server, CLI and desktop apps share one source of truth.

use std::collections::BTreeMap;
use std::sync::OnceLock;

use fluent_bundle::concurrent::FluentBundle;
use fluent_bundle::{FluentArgs, FluentResource, FluentValue};
use serde::{Deserialize, Serialize};
use unic_langid::langid;

const EN_FTL: &str = include_str!("../../../locales/en/odg.ftl");
const PT_FTL: &str = include_str!("../../../locales/pt-BR/odg.ftl");

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum Lang {
    #[default]
    #[serde(rename = "en")]
    En,
    #[serde(rename = "pt-BR")]
    PtBr,
}

impl Lang {
    /// Accepts "pt", "pt-BR", "pt_BR.UTF-8", "en-US", ... Unknown values fall back to English.
    pub fn parse(s: &str) -> Lang {
        if s.trim().to_ascii_lowercase().starts_with("pt") {
            Lang::PtBr
        } else {
            Lang::En
        }
    }

    pub fn code(self) -> &'static str {
        match self {
            Lang::En => "en",
            Lang::PtBr => "pt-BR",
        }
    }

    /// Language from ODG_LANG, then LC_ALL / LC_MESSAGES / LANG.
    pub fn from_env() -> Lang {
        for var in ["ODG_LANG", "LC_ALL", "LC_MESSAGES", "LANG"] {
            if let Ok(v) = std::env::var(var) {
                if !v.is_empty() && v != "C" && v != "POSIX" {
                    return Lang::parse(&v);
                }
            }
        }
        Lang::En
    }
}

struct Catalog {
    bundle: FluentBundle<FluentResource>,
    ids: Vec<String>,
}

fn build(lang: Lang) -> Catalog {
    let (id, src) = match lang {
        Lang::En => (langid!("en"), EN_FTL),
        Lang::PtBr => (langid!("pt-BR"), PT_FTL),
    };
    let res = FluentResource::try_new(src.to_string()).expect("invalid .ftl resource");
    let ids = res
        .entries()
        .filter_map(|e| match e {
            fluent_syntax::ast::Entry::Message(m) => Some(m.id.name.to_string()),
            _ => None,
        })
        .collect();
    let mut bundle = FluentBundle::new_concurrent(vec![id]);
    bundle.set_use_isolating(false);
    bundle.add_resource(res).expect("duplicate .ftl message");
    Catalog { bundle, ids }
}

fn catalog(lang: Lang) -> &'static Catalog {
    static EN: OnceLock<Catalog> = OnceLock::new();
    static PT: OnceLock<Catalog> = OnceLock::new();
    match lang {
        Lang::En => EN.get_or_init(|| build(Lang::En)),
        Lang::PtBr => PT.get_or_init(|| build(Lang::PtBr)),
    }
}

/// Argument value for [`t`].
pub enum Arg<'a> {
    Str(&'a str),
    Num(f64),
}

impl<'a> From<&'a str> for Arg<'a> {
    fn from(s: &'a str) -> Self {
        Arg::Str(s)
    }
}

impl<'a> From<&'a String> for Arg<'a> {
    fn from(s: &'a String) -> Self {
        Arg::Str(s.as_str())
    }
}

impl From<usize> for Arg<'_> {
    fn from(n: usize) -> Self {
        Arg::Num(n as f64)
    }
}

impl From<u32> for Arg<'_> {
    fn from(n: u32) -> Self {
        Arg::Num(n as f64)
    }
}

impl From<f64> for Arg<'_> {
    fn from(n: f64) -> Self {
        Arg::Num(n)
    }
}

/// Translate `key` with named arguments. Missing keys fall back to English, then to the key.
pub fn t(lang: Lang, key: &str, args: &[(&str, Arg)]) -> String {
    let mut fa = FluentArgs::new();
    for (k, v) in args {
        match v {
            Arg::Str(s) => fa.set(*k, FluentValue::from(s.to_string())),
            Arg::Num(n) => fa.set(*k, FluentValue::from(*n)),
        }
    }
    for l in [lang, Lang::En] {
        let cat = catalog(l);
        if let Some(msg) = cat.bundle.get_message(key) {
            if let Some(pattern) = msg.value() {
                let mut errors = vec![];
                return cat.bundle.format_pattern(pattern, Some(&fa), &mut errors).into_owned();
            }
        }
    }
    key.to_string()
}

/// Every message without arguments resolved, for the desktop apps (`ui-*` keys and more).
pub fn all_strings(lang: Lang) -> BTreeMap<String, String> {
    catalog(lang).ids.iter().map(|id| (id.clone(), t(lang, id, &[]))).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn translates_both_languages() {
        let en = t(Lang::En, "problem-W_ORPHAN_NODE", &[("label", "X".into())]);
        let pt = t(Lang::PtBr, "problem-W_ORPHAN_NODE", &[("label", "X".into())]);
        assert_eq!(en, "\"X\" is not connected to anything.");
        assert_eq!(pt, "\"X\" não está conectado a nada.");
    }

    #[test]
    fn plural_and_fallback() {
        assert_eq!(t(Lang::En, "summary-problems", &[("warnings", 1usize.into())]), "1 warning.");
        assert_eq!(t(Lang::PtBr, "summary-problems", &[("warnings", 3usize.into())]), "3 avisos.");
        assert_eq!(t(Lang::PtBr, "does-not-exist", &[]), "does-not-exist");
    }

    #[test]
    fn locales_have_the_same_keys() {
        let en: Vec<_> = all_strings(Lang::En).into_keys().collect();
        let pt: Vec<_> = all_strings(Lang::PtBr).into_keys().collect();
        assert_eq!(en, pt);
    }

    #[test]
    fn parses_lang_codes() {
        assert_eq!(Lang::parse("pt_BR.UTF-8"), Lang::PtBr);
        assert_eq!(Lang::parse("en-US"), Lang::En);
    }
}

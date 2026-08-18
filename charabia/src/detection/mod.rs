pub use script_language::{Language, Script};
use whatlang::Detector;

// file copy pasted from whatlang.
#[allow(dead_code)]
mod chars;
mod script_language;

pub struct StrDetection<'o, 'al> {
    inner: &'o str,
    pub script: Option<Script>,
    pub language: Option<Language>,
    allow_list: Option<&'al [Language]>,
}

impl<'o, 'al> StrDetection<'o, 'al> {
    pub fn new(inner: &'o str, allow_list: Option<&'al [Language]>) -> Self {
        Self { inner, script: None, language: None, allow_list }
    }

    pub fn script(&mut self) -> Script {
        let inner = self.inner;
        *self.script.get_or_insert_with(|| Self::detect_script(inner))
    }

    pub fn language(&mut self) -> Option<Language> {
        let inner = self.inner;
        self.language = match self.language.take() {
            Some(lang) => Some(lang),
            None => match self.allow_list {
                Some([unique_language]) => Some(*unique_language),
                None if Self::detect_script(inner) == Script::Latin => None,
                _otherwise => Self::detect_lang(inner, self.allow_list),
            },
        };

        self.language
    }

    /// detect script with whatlang,
    /// if no script is detected, return Script::Other
    fn detect_script(text: &str) -> Script {
        whatlang::detect_script(text).map(Script::from).unwrap_or_default()
    }

    /// detect lang with whatlang
    /// if no language is detected, return Language::Other
    fn detect_lang(text: &str, allow_list: Option<&[Language]>) -> Option<Language> {
        let detector = allow_list
            .map(|allow_list| allow_list.iter().filter_map(|lang| lang.whatlang()).collect())
            .map(Detector::with_allowlist)
            .unwrap_or_default();

        let detected = detector.detect_lang(text)?;
        // За одним профилем whatlang может стоять не один наш язык: нюнорск
        // опознаётся профилем букмола. Ответ возвращается в тот список, из
        // которого выбирали, иначе названная локаль подменялась бы соседней, у
        // которой ни словаря, ни отношения к тексту. Наименьший из подошедших —
        // чтобы ответ не зависел от порядка списка.
        match allow_list {
            Some(allow_list) => allow_list
                .iter()
                .copied()
                .filter(|language| language.whatlang() == Some(detected))
                .min()
                .or_else(|| Some(Language::from(detected))),
            None => Some(Language::from(detected)),
        }
    }
}

pub trait Detect<'o, 'al> {
    fn detect(&'o self, allow_list: Option<&'al [Language]>) -> StrDetection<'o, 'al>;
}

impl<'o, 'al> Detect<'o, 'al> for &str {
    fn detect(&'o self, allow_list: Option<&'al [Language]>) -> StrDetection<'o, 'al> {
        StrDetection::new(self, allow_list)
    }
}

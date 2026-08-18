use std::collections::HashSet;
use std::sync::LazyLock;

use fst::Set;

use super::{Normalize, Normalizer, NormalizerOption};
use crate::{SeparatorKind, Token, TokenKind};

/// Classify a Token as a word, a stop_word or a separator.
///
/// Assign to each [`Token`]s a [`TokenKind`] using provided stop words.
///
/// [`TokenKind`]: crate::TokenKind
///
/// Any `Token` that is in the stop words [`Set`] is assigned to [`TokenKind::StopWord`].
///
/// [`TokenKind::StopWord`]: crate::TokenKind#StopWord
pub struct Classifier;

impl Normalizer for Classifier {
    fn normalize<'o>(&self, mut token: Token<'o>, options: &NormalizerOption) -> Token<'o> {
        token.kind = TokenKind::Word;
        let lemma = token.lemma();

        if let Some(stop_words) = &options.classifier.stop_words {
            if stop_words.contains(lemma) {
                token.kind = TokenKind::StopWord;
                return token;
            }
        }

        match options.classifier.separators {
            Some(separators) if separators.contains(&lemma) => {
                token.kind = TokenKind::Separator(separator_kind(lemma));
            }
            None if DEFAULT_SEPARATOR_SET.contains(lemma) => {
                token.kind = TokenKind::Separator(separator_kind(lemma));
            }
            _otherwise => (),
        }

        token
    }

    fn should_normalize(&self, token: &Token) -> bool {
        token.kind == TokenKind::Unknown
    }
}

/// Нормализация, в которой хранится стоп-лист: только нелоссовые нормализаторы —
/// ровно то, через что вызывающая сторона проводит стоп-слово перед записью.
/// Регистр она не трогает, так что сравнение остаётся ровно настолько
/// регистрозависимым, насколько им был сам классификатор.
const STOP_WORD_NORMALIZER_OPTION: NormalizerOption = NormalizerOption {
    create_char_map: false,
    lossy: false,
    classifier: ClassifierOption { stop_words: None, separators: None },
    lemmatizer: None,
};

/// Помечает токен стоп-словом, если стоп-словом оказалась его лемма.
///
/// [`Classifier`] работает до лемматизации и видит только словоформу — так
/// продолжают работать стоп-слова, записанные словоформой. Стоп-лист,
/// записанный леммами (для языка с богатой морфологией это самый естественный
/// способ его написать), виден только отсюда.
///
/// Лемма сравнивается в том же виде, в каком лежит стоп-лист: словарь отвечает
/// в своей форме композиции, а стоп-слово записано нелоссово нормализованным,
/// поэтому обе стороны проходят одни и те же нормализаторы. Регистр не
/// приводится ни с одной стороны: словарь отдаёт лемму в своём регистре, и
/// именно он должен сойтись с тем, что написали в настройке.
pub(super) fn classify_lemma<'o>(mut token: Token<'o>, options: &NormalizerOption) -> Token<'o> {
    let Some(stop_words) = &options.classifier.stop_words else {
        return token;
    };

    let is_stop_word = {
        let lemma = token.lemma().normalize(&STOP_WORD_NORMALIZER_OPTION);
        stop_words.contains(lemma.as_ref())
    };
    if is_stop_word {
        token.kind = TokenKind::StopWord;
    }

    token
}

/// Structure for providing options to the classfier.
#[derive(Debug, Clone, Default)]
pub struct ClassifierOption<'no> {
    pub stop_words: Option<Set<&'no [u8]>>,
    pub separators: Option<&'no [&'no str]>,
}

fn separator_kind(lemma: &str) -> SeparatorKind {
    if CONTEXT_SEPARATOR_SET.contains(lemma) {
        SeparatorKind::Hard
    } else {
        SeparatorKind::Soft
    }
}

pub static DEFAULT_SEPARATOR_SET: LazyLock<HashSet<&str>> =
    LazyLock::new(|| crate::separators::DEFAULT_SEPARATORS.iter().copied().collect());

pub static CONTEXT_SEPARATOR_SET: LazyLock<HashSet<&str>> =
    LazyLock::new(|| crate::separators::CONTEXT_SEPARATORS.iter().copied().collect());

#[cfg(test)]
mod test {
    use std::borrow::Cow;

    use crate::normalizer::test::test_normalizer;

    // base tokens to normalize.
    fn tokens() -> Vec<Token<'static>> {
        vec![
            Token { lemma: Cow::Borrowed(" "), ..Default::default() },
            Token { lemma: Cow::Borrowed("\""), ..Default::default() },
            Token { lemma: Cow::Borrowed("@"), ..Default::default() },
            Token { lemma: Cow::Borrowed("."), ..Default::default() },
            Token { lemma: Cow::Borrowed(". "), ..Default::default() },
            Token { lemma: Cow::Borrowed("。"), ..Default::default() },
            Token { lemma: Cow::Borrowed("S.O.S"), ..Default::default() },
            Token { lemma: Cow::Borrowed("ь"), ..Default::default() },
        ]
    }

    // expected result of the current Normalizer.
    fn normalizer_result() -> Vec<Token<'static>> {
        vec![
            Token {
                lemma: Cow::Borrowed(" "),
                kind: TokenKind::Separator(SeparatorKind::Soft),
                ..Default::default()
            },
            Token {
                lemma: Cow::Borrowed("\""),
                kind: TokenKind::Separator(SeparatorKind::Soft),
                ..Default::default()
            },
            Token {
                lemma: Cow::Borrowed("@"),
                kind: TokenKind::Separator(SeparatorKind::Soft),
                ..Default::default()
            },
            Token {
                lemma: Cow::Borrowed("."),
                kind: TokenKind::Separator(SeparatorKind::Soft),
                ..Default::default()
            },
            Token {
                lemma: Cow::Borrowed(". "),
                kind: TokenKind::Separator(SeparatorKind::Hard),
                ..Default::default()
            },
            Token {
                lemma: Cow::Borrowed("。"),
                kind: TokenKind::Separator(SeparatorKind::Hard),
                ..Default::default()
            },
            Token { lemma: Cow::Borrowed("S.O.S"), kind: TokenKind::Word, ..Default::default() },
            Token { lemma: Cow::Borrowed("ь"), kind: TokenKind::Word, ..Default::default() },
        ]
    }

    // expected result of the complete Normalizer pieline.
    fn normalized_tokens() -> Vec<Token<'static>> {
        vec![
            Token {
                lemma: Cow::Borrowed(" "),
                kind: TokenKind::Separator(SeparatorKind::Soft),
                ..Default::default()
            },
            Token {
                lemma: Cow::Borrowed("\""),
                kind: TokenKind::Separator(SeparatorKind::Soft),
                ..Default::default()
            },
            Token {
                lemma: Cow::Borrowed("@"),
                kind: TokenKind::Separator(SeparatorKind::Soft),
                ..Default::default()
            },
            Token {
                lemma: Cow::Borrowed("."),
                kind: TokenKind::Separator(SeparatorKind::Soft),
                ..Default::default()
            },
            Token {
                lemma: Cow::Borrowed(". "),
                kind: TokenKind::Separator(SeparatorKind::Hard),
                ..Default::default()
            },
            Token {
                lemma: Cow::Borrowed("。"),
                kind: TokenKind::Separator(SeparatorKind::Hard),
                ..Default::default()
            },
            Token { lemma: Cow::Borrowed("S.O.S"), kind: TokenKind::Word, ..Default::default() },
            Token { lemma: Cow::Borrowed("ь"), kind: TokenKind::Word, ..Default::default() },
        ]
    }

    test_normalizer!(Classifier, tokens(), normalizer_result(), normalized_tokens());

    #[test]
    fn stop_words() {
        let stop_words = Set::from_iter(["the"].iter()).unwrap();
        let stop_words = stop_words.as_fst().as_bytes();
        let stop_words = Set::new(stop_words).unwrap();
        let options = NormalizerOption {
            create_char_map: true,
            classifier: ClassifierOption { stop_words: Some(stop_words), separators: None },
            lossy: false,
            ..Default::default()
        };

        let token = Classifier
            .normalize(Token { lemma: Cow::Borrowed("the"), ..Default::default() }, &options);
        assert!(token.is_stopword());

        let token = Classifier
            .normalize(Token { lemma: Cow::Borrowed("The"), ..Default::default() }, &options);
        assert!(token.is_word());

        let token = Classifier
            .normalize(Token { lemma: Cow::Borrowed("foobar"), ..Default::default() }, &options);
        assert!(token.is_word());
    }

    /// Словарь на несколько слов: чтобы проверить стоп-лист, лемматизатору
    /// хватает пары ответов, зато они разобраны руками.
    ///
    /// Финская пара отвечает в NFC, тогда как токен доезжает сюда разложенным
    /// нормализатором совместимости — ровно так ведёт себя udlex.
    #[derive(Debug)]
    struct Dictionary;

    impl crate::normalizer::Lemmatizer for Dictionary {
        fn lemma<'o>(
            &self,
            word: &'o str,
            _language: Option<crate::Language>,
            _sentence_initial: bool,
        ) -> Option<Cow<'o, str>> {
            match word {
                "мыла" | "Мыла" | "мыл" => Some(Cow::Borrowed("мыть")),
                "etta\u{308}" => Some(Cow::Borrowed("että")),
                _ => None,
            }
        }
    }

    /// Стоп-лист в том виде, в каком его кладёт Meilisearch: нелоссовая
    /// нормализация, без приведения к нижнему регистру.
    fn stop_word_set(words: &[&str]) -> Vec<u8> {
        let mut normalized: Vec<String> = words
            .iter()
            .map(|word| word.normalize(&NormalizerOption::default()).into_owned())
            .collect();
        normalized.sort();
        Set::from_iter(normalized).unwrap().as_fst().as_bytes().to_vec()
    }

    /// Токен, прошедший весь конвейер вместе со словарём.
    fn lemmatized<'o>(text: &'o str, stop_words: &[u8]) -> Token<'o> {
        let options = NormalizerOption {
            create_char_map: true,
            lossy: true,
            classifier: ClassifierOption {
                stop_words: Some(Set::new(stop_words).unwrap()),
                separators: None,
            },
            lemmatizer: Some(&Dictionary),
        };

        Token { lemma: Cow::Borrowed(text), ..Default::default() }.normalize(&options)
    }

    #[test]
    fn stop_word_written_as_a_lemma_catches_every_word_form() {
        let stop_words = stop_word_set(&["мыть"]);

        // Словоформа, лемма которой объявлена стоп-словом.
        assert!(lemmatized("мыла", &stop_words).is_stopword());
        // Она же с заглавной: словарь отдаёт лемму в нижнем регистре, так что
        // сравнение сходится с тем, что написали в настройке.
        assert!(lemmatized("Мыла", &stop_words).is_stopword());
        // Сама лемма — её ловит ещё классификатор, до словаря.
        assert!(lemmatized("мыть", &stop_words).is_stopword());
        // Слово, которого словарь не знает, стоп-словом не становится.
        assert!(lemmatized("раму", &stop_words).is_word());
    }

    #[test]
    fn stop_word_written_as_a_word_form_keeps_working() {
        let stop_words = stop_word_set(&["мыла"]);

        // Ровно та словоформа, что записана: её разбирает классификатор, как
        // и в стоке, — до словаря дело не доходит.
        assert!(lemmatized("мыла", &stop_words).is_stopword());
        // Другая словоформа того же слова стоп-словом не объявлена, и её лемма
        // в списке не лежит.
        let token = lemmatized("мыл", &stop_words);
        assert!(token.is_word());
        assert_eq!(token.lemma(), "мыть");
    }

    #[test]
    fn lemma_and_stop_word_are_compared_in_the_same_composition_form() {
        // Стоп-слово хранится разложенным, словарь отвечает составленным —
        // сойтись они обязаны всё равно.
        let stop_words = stop_word_set(&["että"]);

        assert!(lemmatized("että", &stop_words).is_stopword());
        assert!(lemmatized("etta\u{308}", &stop_words).is_stopword());
    }

    #[quickcheck]
    fn is_stop_word_iff_stop_words_contain_lemma(
        mut stop_words: Vec<String>,
        lemma: String,
        create_char_map: bool,
        lossy: bool,
        containing: bool,
    ) {
        if containing {
            stop_words.push(lemma.clone());
        } else {
            stop_words.retain(|w| w != &lemma);
        }

        stop_words.sort();
        let stop_words = Set::from_iter(stop_words.iter()).unwrap();
        let stop_words = stop_words.as_fst().as_bytes();
        let stop_words = Set::new(stop_words).unwrap();
        let options = NormalizerOption {
            create_char_map,
            classifier: ClassifierOption { stop_words: Some(stop_words), separators: None },
            lossy,
            ..Default::default()
        };

        let token = Classifier
            .normalize(Token { lemma: Cow::Borrowed(&lemma), ..Default::default() }, &options);
        assert_eq!(token.is_stopword(), containing);
    }

    #[quickcheck]
    fn is_separator_if_separators_contain_lemma(
        mut separators: Vec<String>,
        lemma: String,
        create_char_map: bool,
        lossy: bool,
        containing: bool,
    ) {
        if containing {
            separators.push(lemma.clone());
        } else {
            separators.retain(|w| w != &lemma);
        }
        let separators: Vec<&str> = separators.iter().map(|s| s.as_str()).collect();
        let options = NormalizerOption {
            create_char_map,
            classifier: ClassifierOption { stop_words: None, separators: Some(&separators) },
            lossy,
            ..Default::default()
        };

        let token = Classifier
            .normalize(Token { lemma: Cow::Borrowed(&lemma), ..Default::default() }, &options);
        assert_eq!(token.is_separator(), containing);
        if containing {
            assert!(token.is_separator());
        }
    }

    #[quickcheck]
    fn is_stop_word_if_both_stop_works_and_separators_contain_lemma(
        mut stop_words_and_separators: Vec<String>,
        lemma: String,
        create_char_map: bool,
        lossy: bool,
    ) {
        stop_words_and_separators.push(lemma.clone());
        stop_words_and_separators.sort();
        let stop_words = Set::from_iter(stop_words_and_separators.iter()).unwrap();
        let stop_words = stop_words.as_fst().as_bytes();
        let stop_words = Set::new(stop_words).unwrap();
        let separators: Vec<&str> = stop_words_and_separators.iter().map(|s| s.as_str()).collect();
        let options = NormalizerOption {
            create_char_map,
            classifier: ClassifierOption {
                stop_words: Some(stop_words),
                separators: Some(&separators),
            },
            lossy,
            ..Default::default()
        };

        let token = Classifier
            .normalize(Token { lemma: Cow::Borrowed(&lemma), ..Default::default() }, &options);
        assert!(token.is_stopword());
        assert!(!token.is_separator());
    }
}

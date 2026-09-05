//! Два инварианта, на которых стоит подсветка:
//! куски сегментатора склеиваются в исходную строку, а карта символов токена
//! весит ровно столько байт, сколько занимает токен в оригинале.

use charabia::{Language, Segment, TokenizerBuilder};

fn check(text: &str, allow_list: Option<&[Language]>) -> Result<(), String> {
    let mut builder = TokenizerBuilder::default();
    if let Some(list) = allow_list {
        builder.allow_list(list);
    }
    let tokenizer = builder.build();

    let glued: String = tokenizer.segment_str(text).collect();
    if glued != text {
        return Err(format!("склейка разошлась: {} байт против {}", glued.len(), text.len()));
    }

    let mut previous_end = 0;
    for token in tokenizer.segment(text) {
        if !text.is_char_boundary(token.byte_start) || !text.is_char_boundary(token.byte_end) {
            return Err(format!("границы {}..{} не по символам", token.byte_start, token.byte_end));
        }
        if token.byte_start != previous_end {
            return Err(format!("разрыв: {} != {}", token.byte_start, previous_end));
        }
        previous_end = token.byte_end;
    }
    if previous_end != text.len() {
        return Err(format!("хвост не покрыт: {previous_end} из {}", text.len()));
    }

    for token in tokenizer.tokenize(text) {
        if let Some(map) = &token.char_map {
            let original: usize = map.iter().map(|(origin, _)| *origin as usize).sum();
            if original != token.byte_end - token.byte_start {
                return Err(format!(
                    "карта весит {original} байт, токен {}..{}",
                    token.byte_start, token.byte_end
                ));
            }
        }
    }
    Ok(())
}

fn main() {
    // диапазоны, где живут все специальные сегментаторы и хитрая нормализация
    const RANGES: &[(u32, u32)] = &[
        (0x0020, 0x007e),  // латиница
        (0x0300, 0x036f),  // комбинирующие
        (0x0400, 0x04ff),  // кириллица
        (0x0370, 0x03ff),  // греческий
        (0x0590, 0x05ff),  // иврит
        (0x0600, 0x06ff),  // арабский
        (0x0e00, 0x0e7f),  // тайский
        (0x1780, 0x17ff),  // кхмерский
        (0x3040, 0x30ff),  // кана
        (0x4e00, 0x4fff),  // хань
        (0xac00, 0xd7a3),  // хангыль
        (0xfb00, 0xfb4f),  // лигатуры
        (0xff01, 0xff9f),  // полуширина
        (0xfe00, 0xfe0f),  // селекторы начертания
        (0x1f300, 0x1f64f), // эмодзи
        (0x1f1e6, 0x1f1ff), // флаги
        (0x200b, 0x200d),  // нулевой ширины
    ];

    let lists: [Option<&[Language]>; 5] = [
        None,
        Some(&[Language::Rus, Language::Eng, Language::Pol, Language::Kaz, Language::Ukr]),
        Some(&[Language::Ukr]),
        Some(&[Language::Jpn, Language::Deu]),
        Some(&[Language::Cmn, Language::Kor, Language::Tha, Language::Khm, Language::Ara]),
    ];

    let mut seed = 0x2026_09_04u64;
    let mut random = move || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        seed
    };

    let mut failures = 0;
    for _ in 0..40_000 {
        let mut text = String::new();
        for _ in 0..(random() % 60 + 1) {
            let (low, high) = RANGES[(random() % RANGES.len() as u64) as usize];
            let point = low + (random() % (high - low + 1) as u64) as u32;
            if let Some(c) = char::from_u32(point) {
                text.push(c);
            }
        }
        for list in &lists {
            if let Err(reason) = check(&text, *list) {
                failures += 1;
                if failures <= 8 {
                    println!("{reason}\n  список: {list:?}\n  текст: {text:?}\n  байты: {:?}\n",
                             text.as_bytes());
                }
            }
        }
    }
    println!("провалов: {failures}");
}

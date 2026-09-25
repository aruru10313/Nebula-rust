//! 한글 폰트 로딩 (tofu □□□ 방지)
//!
//! egui 기본 폰트(Ubuntu-Light)에는 한글이 없어서, NotoSansKR 서브셋을
//! Proportional/Monospace 체인 맨 앞에 넣는다.
//! 나머지 문자(라틴/이모지/기호)는 기존 폴백(Ubuntu/NotoEmoji/emoji-icon-font)이 담당.
//! `REQUIRED_CHARS` 커버리지 테스트가 tofu 회귀를 막는다.

/// NotoSansKR 서브셋 (OFL, wght=400 인스턴싱 + 한글/라틴/기호만)
/// 원본: https://github.com/google/fonts (ofl/notosanskr)
pub fn korean_font_bytes() -> &'static [u8] {
    include_bytes!("../../assets/fonts/NotoSansKR-subset.ttf")
}

pub const KOREAN_FONT_NAME: &str = "NotoSansKR";

pub fn install_korean_fonts(ctx: &egui::Context) {
    let mut defs = egui::FontDefinitions::default();
    defs.font_data.insert(
        KOREAN_FONT_NAME.to_owned(),
        std::sync::Arc::new(egui::FontData::from_static(korean_font_bytes())),
    );
    for family in [egui::FontFamily::Proportional, egui::FontFamily::Monospace] {
        if let Some(list) = defs.families.get_mut(&family) {
            list.retain(|name| name != KOREAN_FONT_NAME);
            list.insert(0, KOREAN_FONT_NAME.to_owned());
        }
    }
    ctx.set_fonts(defs);
}

/// UI에서 쓰는 한글 외 특수문자 전수 목록 (tofu 감시 대상)
/// 새 장식 문자를 추가하면 여기에도 넣을 것.
#[cfg(test)]
pub const REQUIRED_CHARS: &[char] = &[
    // 구두점/선
    '·', '—', '→', '─', '×', '…', '–', '＋', // 도형/별 (네뷸랴 장식)
    '▦', '▶', '◆', '★', '☆', '≡', '⬇', '●', '○', // 상태/알림 기호
    '☕', '⚙', '⚠', '⚪', '✅', '⏸', // 이모지 (NotoEmoji 폴백 담당)
    '🌐', '👤', '💾', '📁', '📂', '🔍', '🔭', '🚀',
    // CJK (모드 설명 등 외부 텍스트 대비)
    '回', '次', // 한글/라틴 표본
    '한', '글', '런', '처', '깋', '뷁', '힣', '가', 'A', 'z', '0',
];

#[cfg(test)]
mod tests {
    use super::*;
    use ab_glyph::Font as _;

    fn covers(font_bytes: &[u8], ch: char) -> bool {
        let face = match ab_glyph::FontRef::try_from_slice(font_bytes) {
            Ok(f) => f,
            Err(_) => return false,
        };
        face.glyph_id(ch) != ab_glyph::GlyphId(0)
    }

    #[test]
    fn korean_font_loads() {
        assert!(!korean_font_bytes().is_empty(), "번들 한글 폰트가 비어있음");
        assert!(
            ab_glyph::FontRef::try_from_slice(korean_font_bytes()).is_ok(),
            "번들 한글 폰트 파싱 실패"
        );
    }

    /// 한글 음절 11,172자 전체가 번들 폰트에 있는지 검증
    #[test]
    fn all_hangul_syllables_covered() {
        use ab_glyph::Font as _;
        let face = ab_glyph::FontRef::try_from_slice(korean_font_bytes()).expect("폰트 파싱 실패");
        let mut missing = 0u32;
        for cp in 0xAC00..=0xD7A3 {
            if face.glyph_id(char::from_u32(cp).unwrap()) == ab_glyph::GlyphId(0) {
                missing += 1;
            }
        }
        assert_eq!(missing, 0, "한글 음절 {missing}자 누락");
    }

    /// 체인 전체에서 REQUIRED_CHARS가 하나도 빠짐없이 커버되는지 검증.
    /// 실패하면 해당 문자를 UI에서 교체하거나 폰트 서브셋을 넓힐 것.
    #[test]
    fn no_tofu_in_ui_charset() {
        let mut defs = egui::FontDefinitions::default();
        defs.font_data.insert(
            KOREAN_FONT_NAME.to_owned(),
            std::sync::Arc::new(egui::FontData::from_static(korean_font_bytes())),
        );
        for family in [egui::FontFamily::Proportional, egui::FontFamily::Monospace] {
            if let Some(list) = defs.families.get_mut(&family) {
                list.retain(|name| name != KOREAN_FONT_NAME);
                list.insert(0, KOREAN_FONT_NAME.to_owned());
            }
        }

        for family in [egui::FontFamily::Proportional, egui::FontFamily::Monospace] {
            let chain = defs.families.get(&family).cloned().unwrap_or_default();
            assert!(!chain.is_empty(), "{family:?} 체인이 비어있음");
            let mut missing = vec![];
            for &ch in REQUIRED_CHARS {
                let ok = chain.iter().any(|name| {
                    defs.font_data
                        .get(name)
                        .map(|fd| covers(fd.font.as_ref(), ch))
                        .unwrap_or(false)
                });
                if !ok {
                    missing.push(ch);
                }
            }
            assert!(
                missing.is_empty(),
                "{family:?} 체인에 없는 글리프: {}",
                missing
                    .iter()
                    .map(|c| format!("U+{:04X} {}", *c as u32, c))
                    .collect::<Vec<_>>()
                    .join(", ")
            );
        }
    }
}

//! Static string table for the tray (pt-BR, en, es). No i18n framework on purpose.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Locale {
    PtBr,
    En,
    Es,
}

impl Locale {
    /// Map a BCP 47 / POSIX locale tag; anything unknown falls back to English.
    pub fn parse(tag: &str) -> Self {
        let t = tag.trim().to_ascii_lowercase();
        if t.starts_with("pt") {
            Self::PtBr
        } else if t.starts_with("es") {
            Self::Es
        } else {
            Self::En
        }
    }

    pub fn system() -> Self {
        sys_locale::get_locale().map_or(Self::En, |l| Self::parse(&l))
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Key {
    OpenApp,
    Quit,
    Live,
    LiveCharging,
    LiveVia,
    Stale,
    BleNote,
    NotReported,
    Disconnected,
    TitleCharging,
    LinkUsb,
    LinkDongle,
    LinkBle,
    AgeNow,
    AgeMin,
    AgeHour,
    AgeDay,
}

/// Rows follow `Key` declaration order; columns are `[pt-BR, en, es]`.
pub const KEY_COUNT: usize = 17;
const TABLE: [[&str; 3]; KEY_COUNT] = [
    ["Abrir ATTACK SHARK", "Open ATTACK SHARK", "Abrir ATTACK SHARK"],
    ["Sair", "Quit", "Salir"],
    ["Bateria {level}%", "Battery {level}%", "Batería {level}%"],
    ["Bateria {level}%, carregando", "Battery {level}%, charging", "Batería {level}%, cargando"],
    ["Bateria {level}% ({source})", "Battery {level}% ({source})", "Batería {level}% ({source})"],
    [
        "Última leitura: {level}% via {link}, {age}.",
        "Last reading {level}% via {link}, {age}.",
        "Última lectura: {level}% vía {link}, {age}.",
    ],
    [
        "Este mouse não informa a bateria por Bluetooth.",
        "This mouse does not report battery over Bluetooth.",
        "Este ratón no informa de la batería por Bluetooth.",
    ],
    ["Nível de bateria ainda não informado", "Battery level not reported yet", "Nivel de batería aún no informado"],
    ["Mouse desconectado", "Mouse disconnected", "Ratón desconectado"],
    ["carregando", "charging", "cargando"],
    ["cabo USB", "USB cable", "cable USB"],
    ["receptor 2.4G", "2.4G receiver", "receptor 2.4G"],
    ["Bluetooth", "Bluetooth", "Bluetooth"],
    ["agora há pouco", "just now", "hace un momento"],
    ["há {n} min", "{n} min ago", "hace {n} min"],
    ["há {n} h", "{n} h ago", "hace {n} h"],
    ["há {n} d", "{n} d ago", "hace {n} d"],
];

pub fn tr(loc: Locale, key: Key) -> &'static str {
    TABLE[key as usize][loc as usize]
}

/// Substitute `{name}` placeholders.
pub fn fill(template: &str, args: &[(&str, &str)]) -> String {
    args.iter().fold(template.to_string(), |s, (k, v)| s.replace(&format!("{{{k}}}"), v))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn table_is_complete_and_placeholders_match() {
        assert_eq!(TABLE.len(), KEY_COUNT);
        assert_eq!(Key::AgeDay as usize, KEY_COUNT - 1);
        for row in TABLE {
            assert!(row.iter().all(|s| !s.is_empty()));
            let ph = |s: &str| s.matches('{').count();
            assert!(row.iter().all(|s| ph(s) == ph(row[1])), "placeholder mismatch in {row:?}");
        }
    }

    #[test]
    fn locale_parsing_falls_back_to_en() {
        assert_eq!(Locale::parse("pt-BR"), Locale::PtBr);
        assert_eq!(Locale::parse("pt_PT.UTF-8"), Locale::PtBr);
        assert_eq!(Locale::parse("es-MX"), Locale::Es);
        assert_eq!(Locale::parse("fr-FR"), Locale::En);
        assert_eq!(Locale::parse(""), Locale::En);
    }
}

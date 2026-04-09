use cursive::{
    theme::{
        BaseColor, BorderStyle, Color, Color::TerminalDefault, ColorStyle, PaletteColor::*,
        PaletteStyle, Theme,
    },
    Cursive,
};

pub const COLOR_ERROR: Color = Color::Light(BaseColor::Red);

pub fn apply_theme(siv: &mut Cursive) {
    // let mut theme = siv.current_theme().clone();
    let mut theme = Theme::terminal_default();

    theme.shadow = false;
    theme.borders = BorderStyle::Simple;

    theme.palette[Background] = TerminalDefault; // default bg
    theme.palette[Primary] = TerminalDefault; // default fg
    theme.palette[Shadow] = TerminalDefault;
    theme.palette[View] = TerminalDefault;
    // theme.palette[Primary] = Color::Dark(BaseColor::Yellow);

    theme.palette[Secondary] = TerminalDefault;
    theme.palette[Tertiary] = TerminalDefault;
    // theme.palette[TitlePrimary] = TerminalDefault;
    theme.palette[TitlePrimary] = Color::Dark(BaseColor::Yellow);
    theme.palette[TitleSecondary] = TerminalDefault;
    theme.palette[Highlight] = Color::Light(BaseColor::Yellow);
    theme.palette[HighlightInactive] = Color::Dark(BaseColor::Yellow);
    theme.palette[HighlightText] = Color::Dark(BaseColor::Black);
    theme.palette[PaletteStyle::EditableTextCursor] = ColorStyle::new(
        Color::Dark(BaseColor::Black),
        Color::Light(BaseColor::White),
    )
    .into();

    siv.set_theme(theme);
}

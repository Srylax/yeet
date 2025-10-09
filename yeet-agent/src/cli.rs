use lipgloss::{Color, Style};
use lipgloss_extras::table::Table;

pub fn table() -> Table {
    let style_func = move |row: i32, _col: usize| -> Style {
        match row {
            -1_i32 => Style::new().bold(true).margin_right(2),
            _ => Style::new().margin_right(2),
        }
    };
    Table::new()
        .wrap(true)
        .border_bottom(false)
        .border_left(false)
        .border_right(false)
        .border_top(false)
        .border_column(false)
        .border_row(false)
        .border_style(Style::new().foreground(Color::from("214")))
        .style_func(style_func)
}

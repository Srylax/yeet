pub type Section = (String, Vec<(String, String)>);

pub trait DisplaySection {
    fn as_section(&self) -> Section;
}

macro_rules! section {
    ( $title:expr => [ $( $k:expr, $v:expr ),* $(,)? ] ) => {
        ( $title.to_string(),
            vec![ $( ($k.to_string(), $v.to_string()) ),*]
        )
    };
}
pub(crate) use section;

pub fn print_sections(sections: &[Section]) {
    let width = sections
        .iter()
        .flat_map(|(_, k)| k)
        .map(|(k, _)| k.len())
        .max()
        .unwrap_or(0)
        + 1;
    for (section, items) in sections {
        println!("{section}");

        for (key, value) in items {
            let value = value.to_string();
            // Test if it is a multiline
            if value.lines().count() > 1 {
                let mut lines = value.lines();
                // print first normally key: Value
                println!("{:>w$}: {}", key, lines.next().unwrap(), w = width);

                for line in lines {
                    println!("{:>w$}  {}", "", line, w = width);
                }
            } else {
                println!("{:>w$}: {}", key, value, w = width);
            }
        }
        println!(); // Blank line after section
    }
}

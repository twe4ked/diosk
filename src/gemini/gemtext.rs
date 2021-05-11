// https://gemini.circumlunar.space/docs/gemtext.gmi

pub enum Line {
    Normal(String),
    Link { url: String, name: Option<String> },
}

impl Line {
    pub fn parse(line: &str) -> Line {
        if line.starts_with("=>") {
            // Lines beginning with the two characters "=>" are link lines, which have the following syntax:
            //
            // =>[<whitespace>]<URL>[<whitespace><USER-FRIENDLY LINK NAME>]
            //
            // where:
            //
            //     <whitespace> is any non-zero number of consecutive spaces or tabs
            //     Square brackets indicate that the enclosed content is optional.
            //     <URL> is a URL, which may be absolute or relative.

            let mut last_whitespace = false;
            let mut parts = line
                .splitn(3, |c: char| {
                    if c.is_whitespace() {
                        if last_whitespace {
                            return false;
                        }
                        last_whitespace = true;
                        true
                    } else {
                        last_whitespace = false;
                        false
                    }
                })
                .map(str::trim);

            let _ = parts.next(); // =>

            let url = parts.next().unwrap().to_owned();

            // Name is optional
            let name: String = parts.collect();
            let name = if name.is_empty() { None } else { Some(name) };

            Line::Link { url, name }
        } else {
            Line::Normal(line.to_string())
        }
    }
}

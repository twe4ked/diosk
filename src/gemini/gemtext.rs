// https://gemini.circumlunar.space/docs/gemtext.gmi

#[derive(Debug, PartialEq)]
pub enum Line {
    Normal(String),
    Link { url: String, name: Option<String> },
    InvalidLink,
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
            let split_on_whitespace = {
                |c: char| {
                    if c.is_whitespace() {
                        if last_whitespace {
                            false
                        } else {
                            last_whitespace = true;
                            true
                        }
                    } else {
                        last_whitespace = false;
                        false
                    }
                }
            };
            let mut parts = line.splitn(3, split_on_whitespace).map(str::trim);

            let _ = parts.next(); // =>

            if let Some(url) = parts.next() {
                if url.is_empty() {
                    return Line::InvalidLink;
                }

                // Name is optional
                let name: String = parts.collect();
                let name = if name.is_empty() { None } else { Some(name) };

                Line::Link {
                    url: url.to_string(),
                    name,
                }
            } else {
                Line::InvalidLink
            }
        } else {
            Line::Normal(line.to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn line_parse() {
        let assert_normal = |i: &str, o: &str| {
            assert_eq!(Line::parse(i), Line::Normal(o.to_string()));
        };
        let assert_link = |i: &str, u: &str, n: Option<&str>| {
            assert_eq!(
                Line::parse(i),
                Line::Link {
                    url: u.to_string(),
                    name: n.map(|s| s.to_string()),
                }
            );
        };

        assert_normal(&"", "");
        assert_normal(&"Hello, World", "Hello, World");
        assert_normal(&" => Hello, World", " => Hello, World");

        assert_link(&"=> Hello, World", "Hello,", Some("World"));
        assert_link(&"=>   Hello,   World   ", "Hello,", Some("World"));
    }
}

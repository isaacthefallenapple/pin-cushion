use {
    super::Error,
    fehler::throws,
    lazy_static::lazy_static,
    regex::Regex,
    reqwest::Client,
    std::{fs, io::prelude::*, path::Path},
};

/// Downloads a Pin. `pin_description` is the description
/// field of an `item` in a board's RSS channel. Returns
/// whether it was succesful.
#[throws]
pub async fn download_pin(client: &Client, dir: impl AsRef<Path>, pin_description: &str) -> bool {
    let url_base = match url_base_from_description(pin_description) {
        Some(url) => url,
        None => return false,
    };

    // A Pin's thumbnail is always a .jpg, whereas the original
    // might have any file extension. We iterate over the most
    // common in the hope of getting the right one.
    for &ext in &file_extensions::EXTENSIONS {
        let url = url_base_with_extension(&url_base, ext);
        if let Ok(resp) = client.get(&url).send().await {
            if !resp.status().is_success() {
                continue;
            }
            let body = resp.bytes().await?;
            let file_name = url
                .rsplit('/')
                .next()
                .expect("url needs at least one segment.");

            println!("Getting: {}", file_name);

            let mut file = fs::File::create(&dir.as_ref().join(file_name))?;
            file.write_all(&body[..])?;
            return true;
        }
    }
    false
}

/// Normalises the content from a Pin's description into the
/// URL of the original image sans the file extension.
fn url_base_from_description(descr: &str) -> Option<String> {
    lazy_static! {
        static ref RE: Regex = Regex::new(r#"(?m)img src="(https://i.pinimg.com)/\S*?/(\S*\.).*""#)
            .expect("regex needs to compile");
    }

    RE.captures(descr)
        .map(|caps| format!("{}/originals/{}", &caps[1], &caps[2]))
}

/// Takes a URL normalised with `url_base_from_description` and
/// appends the file extension `ext`.
fn url_base_with_extension(base: &str, ext: file_extensions::Extension) -> String {
    format!("{}{}", base, ext)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_url_replace() {
        let tests = [
            (
                r#"<a href="https://www.pinterest.de/pin/534872893249331184/"> <img src="https://i.pinimg.com/236x/e1/5f/eb/e15feb255af25743320bc495f85e3e28.jpg"></a>"#,
                Some(String::from(
                    r#"https://i.pinimg.com/originals/e1/5f/eb/e15feb255af25743320bc495f85e3e28."#,
                )),
            ),
            (
                r#"<a href="https://www.pinterest.de/pin/534872893249331052/"> <img src="https://i.pinimg.com/236x/80/bb/34/80bb34fc6ed85a445ee5f1b89ffa407e.jpg"></a>"#,
                Some(String::from(
                    r#"https://i.pinimg.com/originals/80/bb/34/80bb34fc6ed85a445ee5f1b89ffa407e."#,
                )),
            ),
            (
                r#"<a href="https://www.pinterest.de/pin/534872893249331052/"></a>"#,
                None,
            ),
            (
                "<a href=\"https://www.pinterest.de/pin/534872893249340309/\">\n                  <img src=\"https://i.pinimg.com/236x/e3/b9/21/e3b9217c7e67f8891d2ff7ba7a0a4fe3.jpg\"></a>",
                Some(String::from("https://i.pinimg.com/originals/e3/b9/21/e3b9217c7e67f8891d2ff7ba7a0a4fe3."))
            ),
        ];

        for (test, want) in &tests {
            assert_eq!(url_base_from_description(test), *want);
        }
    }
}

/// Provides the most common file formats a Pin's image might have.
mod file_extensions {
    use std::fmt;
    use Extension::*;

    /// Contains the most common file extensions.
    pub const EXTENSIONS: [Extension; 8] = [Jpg, Jpeg, Png, Webm, Tiff, Gif, Jfif, Jiff];

    /// Enumerates the most common file extensions. When a new variant is added,
    /// it has to be added to the `EXTENSIONS` array as well.
    #[forbid(dead_code)]
    #[derive(Clone, Copy)]
    pub enum Extension {
        Jpg,
        Jpeg,
        Png,
        Webm,
        Tiff,
        Gif,
        Jfif,
        Jiff,
    }

    impl fmt::Display for Extension {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            use Extension::*;

            let s = match self {
                Jpg => "jpg",
                Jpeg => "jpeg",
                Png => "png",
                Webm => "webm",
                Tiff => "tiff",
                Gif => "gif",
                Jfif => "jfif",
                Jiff => "jiff",
            };

            f.write_str(s)
        }
    }
}

use super::*;

impl Value {
    pub fn format_string(mut fmt: &str, args: Vec<Value>) -> Result<String> {
        let required_argc = fmt.matches('%').count() - 2 * fmt.matches("%%").count();
        if required_argc != args.len() {
            return Err(rterr!(
                "Format string expected {} args, but got {}",
                required_argc,
                args.len()
            ));
        }
        let mut args = args.into_iter();

        let mut ret = String::new();
        while fmt.len() > 0 {
            if fmt.starts_with('%') {
                match fmt[1..].chars().next() {
                    Some('%') => ret.push('%'),
                    Some('s') => ret.push_str(&format!("{}", args.next().unwrap())),
                    Some('r') => ret.push_str(&format!("{:?}", args.next().unwrap())),
                    Some(ch) => {
                        return Err(rterr!(
                            "Invalid format char {:?} (must be one of %, s, or r)",
                            ch
                        ))
                    }
                    None => return Err(rterr!(
                        "Format string ended with a lone '%' (expected a following format character)"
                    )),
                }
                fmt = &fmt[2..];
            } else {
                let i = fmt.find('%').unwrap_or(fmt.len());
                ret.push_str(&fmt[..i]);
                fmt = &fmt[i..];
            }
        }
        Ok(ret)
    }
}

use crate::util::{Error, Result};

use super::Bbq;
use super::{BbqDataType, YamlConvert};
use super::{Command, Type2Data, Type3Data, Type5Data, Type6Data, Type7Data};

fn yaml_error(error_message: &str) -> Error {
    Error::YamlParseError(error_message.to_string())
}

impl YamlConvert for Type2Data {
    fn yaml_lines(&self) -> Vec<String> {
        vec![format!("data: {}", to_array_string(&self.data))]
    }
    fn from_yaml_lines(lines: &[&str], indent: usize) -> Result<Type2Data> {
        if lines.len() != 1 {
            return Err(yaml_error("Type 2 data has more than 1 line!"));
        }
        let data_line = &lines[0][indent..];
        let array = data_line
            .strip_prefix("data: ")
            .ok_or(yaml_error("Type 2 first field is not labelled data!"))?;
        let data = from_array_string(array)?
            .try_into()
            .or(Err(yaml_error("Type 2 data does not have 7 elements!")))?;
        Ok(Type2Data { data })
    }
}

impl YamlConvert for Type3Data {
    fn yaml_lines(&self) -> Vec<String> {
        let mut lines = vec![
            format!("data: {}", to_array_string(&self.data)),
            "children:".to_string(),
        ];
        for child in &self.children {
            lines.push(format!("- {}", to_array_string(child)));
        }
        lines
    }
    fn from_yaml_lines(lines: &[&str], indent: usize) -> Result<Type3Data> {
        if lines.len() < 2 {
            return Err(yaml_error("Type 3 data has less than 2 lines!"));
        }
        let data_line = &lines[0][indent..];
        let array = data_line
            .strip_prefix("data: ")
            .ok_or(yaml_error("Type 3 first field is not labelled data!"))?;
        let data = from_array_string(array)?
            .try_into()
            .or(Err(yaml_error("Type 3 data does not have 7 elements!")))?;

        if &lines[1][2..] != "children:" {
            return Err(yaml_error("Type 3 second field is not labelled children!"));
        }
        let mut children = Vec::new();
        for line in &lines[2..] {
            let array = line[indent..]
                .strip_prefix("- ")
                .ok_or(yaml_error("Type 3 children item does not start with - !"))?;
            let child = from_array_string(array)?
                .try_into()
                .or(Err(yaml_error("Type 3 child does not have 4 elements!")))?;
            children.push(child);
        }
        Ok(Type3Data { data, children })
    }
}

impl YamlConvert for Type5Data {
    fn yaml_lines(&self) -> Vec<String> {
        if let Some(data) = &self.small_data {
            vec![format!("{data}")]
        } else {
            let mut lines = vec!["lines:".to_string()];
            for child in &self.lines {
                lines.push(format!(
                    "- [{}, {}, {}, {}, {}, {}]",
                    child.0, child.1, child.2, child.3, child.4, child.5
                ));
            }
            lines
        }
    }
    fn from_yaml_lines(lines: &[&str], indent: usize) -> Result<Type5Data> {
        if lines.len() == 1 {
            Ok(Type5Data {
                small_data: Some(lines[0][indent..].parse().or(Err(yaml_error(
                    "Type 5 small data could not be parsed as an integer!",
                )))?),
                lines: Vec::new(),
            })
        } else {
            if &lines[0][indent..] != "lines:" {
                return Err(yaml_error("Type 5 first field is not labelled lines!"));
            }

            let mut children = Vec::new();
            for line in &lines[1..] {
                let array = line[indent..]
                    .strip_prefix("- ")
                    .ok_or(yaml_error("Type 5 lines item does not start with - !"))?;
                let child: [u32; 6] = from_array_string(array)?
                    .try_into()
                    .or(Err(yaml_error("Type 5 line does not have 6 elements!")))?;
                children.push((
                    child[0] as u8,
                    child[1] as u8,
                    child[2] as u16,
                    child[3],
                    child[4],
                    child[5],
                ));
            }
            Ok(Type5Data {
                small_data: None,
                lines: children,
            })
        }
    }
}

impl Command {
    fn yaml_line(&self) -> String {
        format!(
            "{{code1: {}, code2: {}, arg_count: {}, place: {}, args: {}}}",
            self.code1,
            self.code2,
            self.arg_count,
            self.place,
            to_array_string(&self.args)
        )
    }
    fn from_yaml_line(line: &str) -> Result<Command> {
        let mut command = line
            .strip_prefix("{")
            .and_then(|s| s.strip_suffix("}"))
            .ok_or(yaml_error("Type 6 command is not surrounded by {}!"))?;

        let code1_str;
        (code1_str, command) = command
            .strip_prefix("code1: ")
            .ok_or(yaml_error("Type 6 command first field is not code1!"))?
            .split_once(", ")
            .ok_or(yaml_error("Type 6 command does not have enough fields!"))?;
        let code1 = code1_str.parse().or(Err(yaml_error(
            "Type 6 command code1 could not be parsed as an integer!",
        )))?;

        let code2_str;
        (code2_str, command) = command
            .strip_prefix("code2: ")
            .ok_or(yaml_error("Type 6 command second field is not code2!"))?
            .split_once(", ")
            .ok_or(yaml_error("Type 6 command does not have enough fields!"))?;
        let code2 = code2_str.parse().or(Err(yaml_error(
            "Type 6 command code2 could not be parsed as an integer!",
        )))?;

        let arg_count_str;
        (arg_count_str, command) = command
            .strip_prefix("arg_count: ")
            .ok_or(yaml_error("Type 6 command third field is not arg_count!"))?
            .split_once(", ")
            .ok_or(yaml_error("Type 6 command does not have enough fields!"))?;
        let arg_count = arg_count_str.parse().or(Err(yaml_error(
            "Type 6 command arg_count could not be parsed as an integer!",
        )))?;

        let place_str;
        (place_str, command) = command
            .strip_prefix("place: ")
            .ok_or(yaml_error("Type 6 command fourth field is not place!"))?
            .split_once(", ")
            .ok_or(yaml_error("Type 6 command does not have enough fields!"))?;
        let place = place_str.parse().or(Err(yaml_error(
            "Type 6 command place could not be parsed as an integer!",
        )))?;

        let args_str = command
            .strip_prefix("args: ")
            .ok_or(yaml_error("Type 6 command fifth field is not args!"))?;
        let args = from_array_string(args_str)?;
        if args.len() != arg_count as usize {
            return Err(yaml_error(
                "Type 6 command args has incorrect number of elements!",
            ));
        }
        Ok(Command {
            code1,
            code2,
            arg_count,
            place,
            args,
        })
    }
}

impl YamlConvert for Type6Data {
    fn yaml_lines(&self) -> Vec<String> {
        let mut lines = vec!["commands:".to_string()];
        for command in &self.commands {
            lines.push(format!("- {}", command.yaml_line()));
        }
        lines
    }
    fn from_yaml_lines(lines: &[&str], indent: usize) -> Result<Type6Data> {
        if &lines[0][indent..] != "commands:" {
            return Err(yaml_error("Type 6 first field is not labelled commands!"));
        }

        let mut commands = Vec::new();
        for line in &lines[1..] {
            let line = line[indent..]
                .strip_prefix("- ")
                .ok_or(yaml_error("Type 6 command item does not start with - !"))?;
            commands.push(Command::from_yaml_line(line)?);
        }
        Ok(Type6Data { commands })
    }
}

impl YamlConvert for Type7Data {
    fn yaml_lines(&self) -> Vec<String> {
        let text_lines: Vec<&str> = self.text.split("\n").collect();
        if text_lines.len() == 1 {
            if text_lines[0].is_empty() {
                vec!["|".to_string()]
            } else {
                vec![text_lines[0].to_string()]
            }
        } else {
            if text_lines.last().map_or(false, |s| s.is_empty()) {
                let mut lines = vec!["|+".to_string()];
                for text_line in &text_lines[..text_lines.len() - 1] {
                    lines.push(text_line.to_string());
                }
                lines
            } else {
                let mut lines = vec!["|-".to_string()];
                for text_line in text_lines {
                    lines.push(text_line.to_string());
                }
                lines
            }
        }
    }
    fn from_yaml_lines(lines: &[&str], indent: usize) -> Result<Type7Data> {
        let first_line = &lines[0][indent..];
        let text = if lines.len() == 1 {
            (if first_line == "|" { "" } else { first_line }).to_string()
        } else {
            let keep_newlines = if first_line == "|-" {
                false
            } else if first_line == "|+" {
                true
            } else {
                return Err(yaml_error(
                    "Type 7 multiline string first line is not |- or |+!",
                ));
            };

            let mut text_lines: Vec<&str> = lines[1..].iter().map(|s| &s[indent..]).collect();
            if !keep_newlines {
                while text_lines.pop_if(|s| s.is_empty()) != None {}
            }
            if keep_newlines {
                text_lines.push("");
            }
            text_lines.join("\n")
        };
        Ok(Type7Data { text })
    }
}

fn to_array_string(data: &[u32]) -> String {
    let line = data
        .iter()
        .map(|a| a.to_string())
        .collect::<Vec<String>>()
        .join(", ");
    format!("[{line}]")
}

fn from_array_string(array: &str) -> Result<Vec<u32>> {
    let array_inner = array
        .strip_prefix("[")
        .and_then(|s| s.strip_suffix("]"))
        .ok_or(yaml_error("Array is not surrounded by []!"))?
        .trim();
    if array_inner.is_empty() {
        Ok(Vec::new())
    } else {
        array_inner
            .split(",")
            .map(|s| {
                s.trim()
                    .parse()
                    .or(Err(yaml_error("Error parsing integer in array!")))
            })
            .collect()
    }
}

fn yamlify<T: BbqDataType>(datas: &[T]) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    for data in datas {
        let data_lines = data.yaml_lines();
        let mut iterator = data_lines.iter();
        match iterator.next() {
            None => continue,
            Some(line) => {
                lines.push(format!("- {line}"));
            }
        }
        while let Some(line) = iterator.next() {
            lines.push(format!("  {line}"));
        }
    }

    lines
}

fn deyamlify_chunk<T: BbqDataType>(lines: &[&str]) -> Result<Vec<T>> {
    lines
        .chunk_by(|_, b| !b.starts_with("- "))
        .map(|chunk| T::from_yaml_lines(chunk, 2))
        .collect()
}

impl Bbq {
    pub fn yaml_lines(&self) -> Vec<String> {
        let mut lines = vec![format!("datetime: {}", self.datetime)];
        if let Some(data) = &&self.type2data {
            lines.push("type2data:".to_string());
            lines.append(&mut yamlify(data));
        }
        if let Some(data) = &&self.type3data {
            lines.push("type3data:".to_string());
            lines.append(&mut yamlify(data));
        }
        if let Some(data) = &&self.type5data {
            lines.push("type5data:".to_string());
            lines.append(&mut yamlify(data));
        }
        if let Some(data) = &&self.type6data {
            lines.push("type6data:".to_string());
            lines.append(&mut yamlify(data));
        }
        if let Some(data) = &&self.type7data {
            lines.push("type7data:".to_string());
            lines.append(&mut yamlify(data));
        }
        if self.footer != [0; 6] {
            lines.push(format!("footer: {}", to_array_string(&self.footer)));
        }
        lines
    }

    pub fn from_yaml_lines(lines: &[&str]) -> Result<Self> {
        let mut line_chunks = lines.chunk_by(|_, b| b.starts_with("- ") || b.starts_with("  "));

        let datetime_chunk = line_chunks.next().ok_or(yaml_error("Yaml has no data!"))?;
        if datetime_chunk.len() != 1 {
            return Err(yaml_error("First yaml chunk has more than one line!"));
        }
        let datetime_string = datetime_chunk[0]
            .strip_prefix("datetime: ")
            .ok_or(yaml_error("First yaml chunk is not labelled datetime!"))?;
        let datetime: u32 = datetime_string.parse().or(Err(yaml_error(
            "Datetime field could not be parsed as integer!",
        )))?;

        let (mut type2data, mut type3data, mut type5data, mut type6data, mut type7data) =
            (None, None, None, None, None);
        let mut footer = [0; 6];
        while let Some(chunk) = line_chunks.next() {
            match chunk[0] {
                "type2data:" => {
                    type2data = Some(deyamlify_chunk(&chunk[1..])?);
                }
                "type3data:" => {
                    type3data = Some(deyamlify_chunk(&chunk[1..])?);
                }
                "type5data:" => {
                    type5data = Some(deyamlify_chunk(&chunk[1..])?);
                }
                "type6data:" => {
                    type6data = Some(deyamlify_chunk(&chunk[1..])?);
                }
                "type7data:" => {
                    type7data = Some(deyamlify_chunk(&chunk[1..])?);
                }
                _ => {
                    if let Some(footer_str) = chunk[0].strip_prefix("footer: ") {
                        if chunk.len() > 1 {
                            return Err(yaml_error("Footer chunk has more than 1 line!"));
                        }
                        footer = from_array_string(footer_str)?
                            .try_into()
                            .or(Err(yaml_error(
                                "Footer array has incorrect number of elements!",
                            )))?;
                    } else {
                        return Err(yaml_error("Unrecognised yaml chunk!"));
                    }
                }
            }
        }
        Ok(Bbq {
            datetime,
            type2data,
            type3data,
            type5data,
            type6data,
            type7data,
            footer,
        })
    }
}

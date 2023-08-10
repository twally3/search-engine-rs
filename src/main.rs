use std::{
	collections::HashMap,
	fs::{self, File},
	io::{BufReader, BufWriter},
	path::Path,
};
use xml::reader::{EventReader, XmlEvent};

#[derive(Debug)]
struct Lexer<'a> {
	content: &'a [char],
}

impl<'a> Lexer<'a> {
	fn new(content: &'a [char]) -> Self {
		return Self { content };
	}

	fn trim_left(&mut self) {
		while self.content.len() > 0 && self.content[0].is_whitespace() {
			self.content = &self.content[1..]
		}
	}

	fn chop(&mut self, n: usize) -> &'a [char] {
		let token = &self.content[0..n];
		self.content = &self.content[n..];
		return token;
	}

	fn chop_while<P>(&mut self, mut prediate: P) -> &'a [char]
	where
		P: FnMut(&char) -> bool,
	{
		let mut n = 0;
		while n < self.content.len() && prediate(&self.content[n]) {
			n += 1;
		}
		return self.chop(n);
	}

	fn next_token(&mut self) -> Option<&'a [char]> {
		self.trim_left();
		if self.content.len() == 0 {
			return None;
		}

		if self.content[0].is_numeric() {
			return Some(self.chop_while(|x| x.is_numeric()));
		}

		if self.content[0].is_alphabetic() {
			return Some(self.chop_while(|x| x.is_alphanumeric()));
		}

		return Some(self.chop(1));
	}
}

impl<'a> Iterator for Lexer<'a> {
	type Item = &'a [char];

	fn next(&mut self) -> Option<Self::Item> {
		return self.next_token();
	}
}

fn read_xml_file<P: AsRef<Path>>(file_path: P) -> std::io::Result<String> {
	let file = File::open(file_path)?;
	let file = BufReader::new(file);
	let reader = EventReader::new(file);
	let mut content = String::new();

	for event in reader.into_iter() {
		if let Ok(XmlEvent::Characters(text)) = event {
			content.push_str(&text);
			content.push_str(" ")
		}
	}

	return Ok(content);
}

type TF = HashMap<String, usize>;
type DTF = HashMap<std::path::PathBuf, TF>;

fn main() -> std::io::Result<()> {
	let dir_path = "transcripts";
	let dir = fs::read_dir(dir_path)?;
	let mut tf_index = DTF::new();

	for file in dir {
		let file_path = file?.path();

		println!("Indexing {file_path:?}");

		let content = read_xml_file(&file_path)?.chars().collect::<Vec<_>>();
		let mut tf = HashMap::<String, usize>::new();

		for token in Lexer::new(&content) {
			let term = token
				.into_iter()
				.map(|x| x.to_ascii_uppercase())
				.collect::<String>();

			tf.entry(term).and_modify(|x| *x += 1).or_insert(1);
		}

		// let mut stats = tf.iter().collect::<Vec<_>>();
		// stats.sort_by_key(|(_, &f)| f);
		// stats.reverse();

		tf_index.insert(file_path, tf);
	}

	let index_path = "index.json";
	let file = File::create(index_path)?;
	let file = BufWriter::new(file);
	serde_json::to_writer(file, &tf_index)?;

	return Ok(());
}

use rust_stemmers::{Algorithm, Stemmer};
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

	fn next_token(&mut self) -> Option<String> {
		self.trim_left();
		if self.content.len() == 0 {
			return None;
		}

		if self.content[0].is_numeric() {
			return Some(self.chop_while(|x| x.is_numeric()).iter().collect());
		}

		if self.content[0].is_alphabetic() {
			let term = self
				.chop_while(|x| x.is_alphanumeric())
				.iter()
				.map(|x| x.to_ascii_lowercase())
				.collect::<String>();

			let en_stemmer = Stemmer::create(Algorithm::English);
			let term = en_stemmer.stem(&term);

			return Some(term.to_string());
		}

		return Some(self.chop(1).iter().collect());
	}
}

impl<'a> Iterator for Lexer<'a> {
	type Item = String;

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

type TermFreq = HashMap<String, usize>;
type TermFreqForDoc = HashMap<std::path::PathBuf, TermFreq>;
type DocFreq = HashMap<String, usize>;

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct Index {
	term_freq_per_doc: TermFreqForDoc,
	doc_freq: DocFreq,
}

impl Index {
	fn new(term_freq_per_doc: TermFreqForDoc, doc_freq: DocFreq) -> Self {
		return Self {
			term_freq_per_doc,
			doc_freq,
		};
	}
}

fn index() -> std::io::Result<()> {
	let dir_path = "transcripts";
	let dir = fs::read_dir(dir_path)?;

	let mut tf_for_docs = TermFreqForDoc::new();
	let mut doc_freq = DocFreq::new();

	for file in dir {
		let file_path = file?.path();

		println!("Indexing {file_path:?}");

		let content = read_xml_file(&file_path)?.chars().collect::<Vec<_>>();
		let mut tf = HashMap::<String, usize>::new();

		for term in Lexer::new(&content) {
			tf.entry(term).and_modify(|x| *x += 1).or_insert(1);
		}

		for (token, _) in tf.iter() {
			doc_freq
				.entry(token.to_owned())
				.and_modify(|x| *x += 1)
				.or_insert(1);
		}

		tf_for_docs.insert(file_path, tf);
	}

	let idx = Index::new(tf_for_docs, doc_freq);

	let index_path = "index.json";
	let file = File::create(index_path)?;
	let file = BufWriter::new(file);
	serde_json::to_writer(file, &idx)?;

	return Ok(());
}

fn search() -> std::io::Result<()> {
	let index_path = "index.json";
	let file = File::open(index_path)?;
	let file = BufReader::new(file);
	let idx: Index = serde_json::from_reader(file)?;

	let manifest: HashMap<String, String> =
		serde_json::from_reader(BufReader::new(File::open("transcripts/manifest.json")?))?;

	let terms = std::env::args().nth(2).expect("no pattern given");
	let terms = terms.chars().collect::<Vec<_>>();

	let mut scores = Vec::<(std::path::PathBuf, f32)>::new();

	let num_docs = idx.term_freq_per_doc.len() as f32;
	println!("Total documents: {num_docs:?}");

	for (path, term_freq) in idx.term_freq_per_doc {
		let total_words_in_doc: usize = term_freq.iter().map(|(_, x)| *x).sum();

		let mut rank = 0.0;

		for term in Lexer::new(&terms) {
			let term_freq_in_doc = match term_freq.get(&term) {
				Some(t) => *t as f32,
				None => 0.0,
			};

			let tf = term_freq_in_doc / total_words_in_doc as f32;

			let total_number_of_docs = num_docs;
			let num_docs_with_term = match idx.doc_freq.get(&term) {
				Some(freq) => *freq as f32,
				None => 0.0,
			};
			let idf = (total_number_of_docs / (num_docs_with_term)).log10();

			let sub_total = tf * idf;
			rank += sub_total;
		}

		scores.push((path, rank));
	}

	scores.sort_by(|(_, a), (_, b)| a.total_cmp(b));
	scores.reverse();

	println!("SEARCH TERM: {search:?}", search = String::from_iter(terms));
	for (k, v) in scores
		.iter()
		.take(10)
		.filter(|(_, score)| score.total_cmp(&0.0).is_gt())
	{
		let id = k
			.file_stem()
			.map(|x| x.to_string_lossy().to_string())
			.and_then(|x| manifest.get(&x));

		let name = id.map(|x| x.to_owned()).unwrap_or_else(|| "N/a".to_owned());

		println!("{name:?} ({id:?}) : {v:?}");
	}

	return Ok(());
}

#[derive(Debug, serde::Deserialize)]
struct YTMeta {
	// author_name: String,
	// author_url: String,
	// height: usize,
	// html: String,
	// provider_name: String,
	// provider_url: String,
	// thumbnail_height: usize,
	// thumbnail_url: String,
	// thumbnail_width: usize,
	title: String,
	// r#type: String,
	// version: String,
	// width: usize,
}

fn find_unnamed() -> std::io::Result<()> {
	let index_path = "index.json";
	let file = File::open(index_path)?;
	let file = BufReader::new(file);
	let idx: Index = serde_json::from_reader(file)?;

	let mut manifest: HashMap<String, String> =
		serde_json::from_reader(BufReader::new(File::open("transcripts/manifest.json")?))?;

	let paths = idx
		.term_freq_per_doc
		.iter()
		.map(|(path, _)| path.file_stem())
		.filter_map(|x| x)
		.map(|x| x.to_string_lossy().to_string())
		.collect::<Vec<String>>();

	for path in paths {
		if !manifest.contains_key(&path) {
			if path == "manifest" {
				continue;
			}

			let url =
				format!("https://www.youtube.com/oembed?url=http://youtube.com/watch?v={path}&format=json");

			let resp = match reqwest::blocking::get(url) {
				Ok(data) => data,
				Err(_) => panic!("Failed to fetch data"),
			};

			if !resp.status().is_success() {
				println!("Skipping {path}");
				continue;
			}

			let data = match resp.json::<YTMeta>() {
				Ok(data) => data,
				Err(e) => panic!("{e:?}"),
			};

			println!("{path} => {x}", x = data.title);
			manifest.insert(path, data.title);
		}
	}

	let file = std::fs::OpenOptions::new()
		.write(true)
		.open("transcripts/manifest.json")?;
	serde_json::to_writer(BufWriter::new(file), &manifest)?;

	return Ok(());
}

fn main() -> std::io::Result<()> {
	let pattern = std::env::args().nth(1).expect("no pattern given");
	return match pattern.as_str() {
		"index" => index(),
		"search" => search(),
		"find_unnamed" => find_unnamed(),
		_ => Err(std::io::Error::new(
			std::io::ErrorKind::InvalidInput,
			"Invalid input".to_owned(),
		)),
	};
}

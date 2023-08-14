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

type TermFreq = HashMap<String, usize>;
type TermFreqForDoc = HashMap<std::path::PathBuf, TermFreq>;
type DocFreq = HashMap<String, usize>;

#[derive(Debug,serde::Serialize,serde::Deserialize)]
struct Index {
    term_freq_per_doc: TermFreqForDoc,
    doc_freq: DocFreq,
}

impl Index {
    fn new(term_freq_per_doc: TermFreqForDoc, doc_freq: DocFreq) -> Self {
        return Self { term_freq_per_doc, doc_freq };
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

		for token in Lexer::new(&content) {
			let term = token
				.into_iter()
				.map(|x| x.to_ascii_uppercase())
				.collect::<String>();

			tf.entry(term).and_modify(|x| *x += 1).or_insert(1);
		}

        for (token, _) in tf.iter() {
            doc_freq.entry(token.to_owned()).and_modify(|x| *x += 1).or_insert(1);
        }

		// let mut stats = tf.iter().collect::<Vec<_>>();
		// stats.sort_by_key(|(_, &f)| f);
		// stats.reverse();

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

    let manifest: HashMap<String, String> = serde_json::from_reader(BufReader::new(File::open("transcripts/manifest.json")?))?;

    // let terms = "legend of zelda";
    let terms = "zelda tears of the kingdom";
    // let terms = "breath of the wild";
    // let terms = "pokemon";
    // let terms = "rust debugging";
    let terms = terms.chars().collect::<Vec<_>>();

    let mut scores = Vec::<(std::path::PathBuf, f32)>::new();

    let num_docs = idx.term_freq_per_doc.len() as f32;
    for (path, term_freq) in idx.term_freq_per_doc {
        let total_words_in_doc: usize = term_freq.iter().map(|(_, x)| *x).sum();
        println!("{path:?} => {total_words_in_doc:?} words");

        let mut rank = 0.0;

        for token in Lexer::new(&terms) {
			let term = token
				.into_iter()
				.map(|x| x.to_ascii_uppercase())
				.collect::<String>();

            let term_freq_in_doc = match term_freq.get(&term) {
                Some(t) => *t as f32,
                None => 0.0
            };

            let tf = term_freq_in_doc / total_words_in_doc as f32;

            let total_number_of_docs = num_docs;
            let num_docs_with_term = match idx.doc_freq.get(&term) {
                Some(freq) => *freq as f32,
                None => 0.0
            };
            let idf = (total_number_of_docs / (num_docs_with_term)).log10();

            let sub_total = tf * idf;
            rank += sub_total;
            println!("{term:?} : {term_freq_in_doc:?}, {num_docs_with_term:?} => {tf:?} / {idf:?} = {sub_total:?}");
        }
        println!("Total for doc: {rank:?}");
        println!("---");

        scores.push((path, rank));
    }

    scores.sort_by(|(_, a),(_, b)| a.total_cmp(b));
    scores.reverse();

    println!("SEARCH TERM: {search:?}", search = String::from_iter(terms) );
    for (k, v) in scores.iter().take(10) {
        let path_string = match k.clone().into_os_string().into_string() {
            Ok(t) => Ok(t),
            Err(_) => Err(std::io::Error::new(std::io::ErrorKind::Unsupported, "Failed to parse path string")),
        }?;
        let id = path_string.split("/").last().unwrap().split(".").next().unwrap();

        let name = match manifest.get(id) {
            Some(x) => x,
            None => "N/a"
        };
        println!("{name:?} ({id:?}) : {v:?}");
    }

    return Ok(());
}

fn main() -> std::io::Result<()> {
    let pattern = std::env::args().nth(1).expect("no pattern given");
    return match pattern.as_str() {
        "index" => index(),
        "search" => search(),
        _ => Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, "Invalid input".to_owned()))
    };
}

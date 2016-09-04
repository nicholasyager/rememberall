extern crate stem;
extern crate rustc_serialize;
extern crate docopt;
extern crate glob;
extern crate crypto;
extern crate csv;
extern crate colored;

use docopt::Docopt;
use self::glob::glob;
use crypto::digest::Digest;
use crypto::sha2::Sha256;
use std::fs;
use std::io::Read;
use std::io::Write;
use std::collections::HashMap;
use std::collections::HashSet;
use colored::*;
use std::env;

const USAGE: &'static str = "
rememberall - More magical than the original.

Usage:
    rememberall index <directory>...
    rememberall search <term>... [-n <docs>]
    rememberall (-h | --help)

Options:
  -h --help     Show this screen.
  -n <docs>     Number of documents to return [default: 1].
";

#[derive(Debug, RustcDecodable)]
struct Args {
    cmd_index: bool,
    cmd_search: bool,
    flag_n: isize,
    arg_directory: Vec<String>,
    arg_term: Vec<String>
}

struct Corpus {
    documents: HashMap<String, Document>,
    terms: HashMap<String, i32>,
}

impl Corpus {
    fn new() -> Corpus {
        return Corpus {
            documents: HashMap::new(),
            terms: HashMap::new()
        }
    }

    fn load_indices(&mut self, home_dir: String) {
        // Load the index.
        let mut index_file = fs::File::open(home_dir+"/.rememberall/index.csv").unwrap();
        let mut index_buffer = String::new();
        let _ = index_file.read_to_string(&mut index_buffer);

        let mut index_csv_reader = csv::Reader::from_string(index_buffer).has_headers(false);

        for row in index_csv_reader.decode() {
            let (id, word, doc_freq, term_freq): (String, String, i32, i32) = row.unwrap();

            match self.documents.get_mut(&id) {
                Some(doc) => doc.terms.insert(word.clone(), doc_freq),
                _ => continue
            };
            &self.terms.insert(word, term_freq);
        }
    }

    fn load_corpus(&mut self, home_dir: String) {
        let mut corpus_file = fs::File::open(home_dir.clone()+"/.rememberall/corpus.csv").unwrap();
        let mut corpus_buffer = String::new();
        let _ = corpus_file.read_to_string(&mut corpus_buffer);
        let mut corpus_csv_reader = csv::Reader::from_string(corpus_buffer).has_headers(false);
        for row in corpus_csv_reader.decode() {
            let (source, title, text, id, length): (String, String, String, String, i32) = row.unwrap();
            self.documents.insert(id, Document {
                title: title,
                source: source,
                text: text.replace("<br><ul>", "\n    *   ").replace("<ul>", "*   ").replace("<br>","\n       "),
                terms: HashMap::new(),
                length: length
            });
        }

    }

    fn load(home_dir: String) -> Corpus {

        let mut corpus: Corpus = Corpus::new();
        corpus.load_corpus(home_dir.clone());
        corpus.load_indices(home_dir);

        return corpus;
    }

    fn load_text(&mut self, path: String)  {
        // Initialize some values
        let mut index = 0;

        // Load the file
        let mut file = fs::File::open(&path).unwrap();
        let mut string_buffer = String::new();
        let _ = file.read_to_string(&mut string_buffer);

        // Start by iterating over the document line by line
        let chunks: Vec<&str> = string_buffer.split('#').collect();

        for chunk in chunks {
            if chunk.len() == 0 {
                continue;
            }
            if index > 0 {
                let mut new_document = Document::parse(chunk.to_string());
                let mut sha = Sha256::new();
                sha.input_str(new_document.text.trim_right());
                if new_document.text.len() > 0 {
                    new_document.source = path.clone();
                    new_document.term_frequency();
                    self.documents.insert(sha.result_str().to_string(), new_document);
                }

            }
            index += 1;
        }
    }

    fn document_frequency(&mut self) {
        for (_, document) in &mut self.documents {

            for (term, _) in &mut document.terms {
                // Check if the string is already present in the map. If not, add it.
                let count: i32 = match self.terms.get(term) {
                    Some(count) => count + 1,
                    _ => 1,
                };
                self.terms.insert(term.to_string(), count);
            }
        }
    }

    fn save(&self, home_dir: String) {
        let mut corpus_file = fs::File::create(home_dir+"/.rememberall/corpus.csv").unwrap();
        for (id, document) in &self.documents {
            let _ = corpus_file.write_fmt(format_args!("\"{}\",\"{}\",\"{}\",\"{}\",\"{}\"\n",
                                          document.source, document.title, document.text, id, document.length));
        }
    }

    fn print(&self) {
        for (_, document) in &self.documents {
            document.print();
        }
    }
}

struct Document {
    title: String,
    source: String,
    text: String,
    terms: HashMap<String, i32>,
    length: i32
}

impl Clone for Document {
    fn clone(&self) -> Document {
        return Document {
            title: self.title.clone(),
            source: self.source.clone(),
            text: self.text.clone(),
            terms: self.terms.clone(),
            length: self.length.clone()
        }
    }
}

impl Document {

    fn parse(content: String) -> Document {

        // Initialize variables
        let mut title = String::new();
        let mut text = String::new();
        let mut index = 0;

        // Split the text by asterisk
        let lines: Vec<&str> = content.split("*  ").collect();

        // Iterate through the lines
        for line in lines {
            if index == 0 {
                // This is the first line. It is the title
                title = line.trim().replace("\"","'").to_string();
            } else {
                // This is the text. If the current text is not empty, add a space.
                let filtered_line = line.replace("    ","\t");
                text.push_str("<ul>");
                text.push_str(filtered_line.trim_left());
            }
            index += 1;
        }

        return Document {
            title: title,
            source: String::new(),
            text: text.trim_right().replace("\n","<br>").replace("\"","'").replace("\t"," ")
                      .to_string().replace('"', "'"),
            terms: HashMap::new(),
            length: 0
        }

    }

    fn term_frequency(&mut self) {
        // Calculate the term frequency for all of the terms in the document.
        let merged = self.text.clone() + " " + &self.title;
        let text = merged.replace("<ul>"," ").replace("["," ").replace("**"," ").replace(":","").to_string();
        let term_list: Vec<&str> = text.split_whitespace().collect();

        for term in term_list.clone() {

            // Clense the terms. Make lowercase and remove extra symobls.
            let clean_term = term.to_lowercase().replace(".", "").replace("\t"," ")
            .replace(",","").replace("\"","").replace("<br>", " ").replace("<li>", "");

            let s = stem::get(&clean_term);
            let stem = match s {
                Ok(stemmed) => stemmed,
                Err(_) => clean_term.clone(),
            };

            // Check if the string is already present in the map. If not, add it.
            let count: i32 = match self.terms.get(&stem) {
                Some(count) => count + 1,
                _ => 1,
            };
            self.terms.insert(stem.to_string(), count);
        }
        self.length = term_list.len() as i32;
    }


    fn print(&self) {
        println!("{}\n\n{}\n", &self.title, &self.text);
    }

}

fn scan_directory(glob_string: String, paths: &mut Vec<String>) {
    for entry in glob(&glob_string).expect("Failed to read glob pattern") {
        match entry {
            Ok(path) =>  {
                match path.to_str() {
                    None => panic!("new path is not a valid UTF-8 sequence"),
                    Some(string) => paths.push(string.to_string()),
                }
            },
            Err(e) => println!("{:?}", e),
        }
    }
}

fn get_value(map: &HashMap<String, f32>, id: &String, default: f32) -> f32 {
    let current_value = match map.get(id) {
        Some(value) => *value,
        None => default
    };
    return current_value
}

fn search(args: Args, home_dir: String) {
    let corpus = Corpus::load(home_dir);
    let number_of_documents : i32 = corpus.documents.len() as i32;

    // Stem the input.
    let mut stems: HashSet<String> = HashSet::new();
    for term in &args.arg_term {
            let s = stem::get(&term);
            match s {
                Ok(stemmed) => stems.insert(stemmed),
                Err(_) => stems.insert(term.clone()),
            };
    }

    // Bayesian Classification
    // For this step we need three things:
    // 1. The prior probability of this document being chosen.
    // 2. The likelihood of the words occuring in the document
    // 3. The evidence. This is the hard part.

    // The Prior
    let prior: f32 = 1_f32 / (number_of_documents as f32);


    // The likelihood.
    // This can be described as the mutliplication of all of the
    // document_term_frequencies for each stem and document.
    let mut likelihoods: HashMap<String, f32> = HashMap::new();
    for stem in &stems {

        for (id, document) in &corpus.documents {
            let document_term_frequency: i32 = match document.terms.get(stem) {
                Some(value) => *value,
                _ => 0
            };
            let current_value = get_value(&likelihoods, id, 1.0_f32);
            if current_value == 0.0 {
                continue;
            }
            let likelihood = (document_term_frequency as f32) / (document.length as f32);
            likelihoods.insert(id.to_string(), current_value * likelihood);
        }
    }

    // The Evidence.
    // For each document, calculate add the likelihood to the inverse of the
    // likelihood. That is, the inverse prior and the probability of
    // obserserving the word in all of the other documents.
    let mut evidences: HashMap<String, f32> = HashMap::new();
    let mut all_words = 0;
    for (_, document) in &corpus.documents {
        all_words += document.length;
    }

    for stem in &stems {
        for (id, _) in &corpus.documents {
            let mut other_occurances = 0;
            for (comparison_id, comparison_document) in &corpus.documents {
                if id == comparison_id {
                    continue;
                }

                let document_term_frequency: i32 = match comparison_document.terms.get(stem) {
                    Some(value) => *value,
                    _ => 0
                };

                other_occurances += document_term_frequency;
            }
            let current_value = get_value(&evidences, id, 1.0_f32);
            if current_value == 0.0 {
                continue;
            }
            let evidence = (other_occurances as f32) / (all_words as f32);
            evidences.insert(id.to_string(), current_value * evidence);
        }
    }

    let mut results: Vec<(String, f32)> = Vec::new();
    for (id, _) in &corpus.documents {
        let likelihood: f32 = get_value(&likelihoods, id, 0.0_f32);
        let evidence: f32 = get_value(&evidences, id, 0.0_f32);
        let probability = prior*likelihood / ((prior*likelihood)+((1.0_f32-prior)*evidence));
        results.push((id.to_string(), probability));
    }

    let mut max_score = 0.0_f32;
    for (_, score) in results.clone() {
        if score > max_score {
            max_score = score;
        }
    }

    if max_score == 0.0_f32 {
        let string: String = "No likely results found.".to_string();
        println!("{}", string.red());
        std::process::exit(1);
    }

    let mut scaled_results: Vec<(String, u32)> = Vec::new();
    for (id, score) in results {
        let rank: u32 = (score * 1000000000.0_f32) as u32;
        scaled_results.push((id,rank));
    }

    scaled_results.sort_by_key(|a| a.1);
    scaled_results.reverse();

    let mut index = 0;

    println!("\n");

    for (id, score) in scaled_results {
        if index >= args.flag_n {
            break;
        }
        match corpus.documents.get(&id) {
            Some(doc) => {
                let score_string: String = ((score as f32)/1000000000.0_f32).to_string();
                println!("{}\n{}\n{}\n\n    {}\n\n",
                        doc.title.green(), doc.source, score_string.yellow(), doc.text)
            },
            _ => continue,
        };

        index +=1;
    }

}

fn index(args: Args, home_dir: String) {
    // Try to create the data directory
    match fs::create_dir(home_dir.clone() + "/.rememberall") {
        Ok(directory) => directory,
        Err(_) => println!("Updating."),
    };

    // Initialize vectors to store the directories and paths
    let mut paths: Vec<String> = Vec::new();
    let directories = args.arg_directory;

    // For each directory, scan for markdown files and add it to the list.
    for directory in directories {
        let glob_string: String = directory + "/*.markdown";
        scan_directory(glob_string, &mut paths);
    }

    // Create a corpus
    let mut corpus = Corpus::new();

    // For each document
    for path in paths {
        // Load each file and parse into documents.
        corpus.load_text(path);

    }
    corpus.document_frequency();
    // Calculate the term frequency inverse document frequency and write to disk
    let mut index_file = fs::File::create(home_dir.clone()+"/.rememberall/index.csv").unwrap();
    //let document_list = corpus.documents.clone();
    for (id, document) in &corpus.documents {
        for (term, frequency) in &document.terms {
            // Get the inverse document frequency from the corpus
            let term_frequency: i32 = match corpus.terms.get(term) {
                Some(count) => *count,
                _ => 0,
            };
            let _ = index_file.write_fmt(format_args!("\"{}\",\"{}\",{},{}\n", id, term, frequency, term_frequency));
        }
    }
    corpus.save(home_dir.clone());
    println!("Index {} documents, {} terms.", corpus.documents.len(), corpus.terms.len());
}

fn main() {
    // Parse the command line arguments to get a list of directories to scan.
    let args: Args = Docopt::new(USAGE)
    .and_then(|d| d.decode())
    .unwrap_or_else(|e| e.exit());

    // Get the user's home directory

    let home_dir: String;

    match env::home_dir() {
        Some(path) => {
            home_dir = path.to_str().unwrap().to_string();
        },
        None => {
            println!("Please set your $HOME variable");
            std::process::exit(0);
        },
    };

    if args.cmd_index {
        index(args, home_dir);
    } else if args.cmd_search {
        search(args, home_dir);
    }

}

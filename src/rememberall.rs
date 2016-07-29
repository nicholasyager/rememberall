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
    terms: HashMap<String, f32>,
}

impl Corpus {
    fn new() -> Corpus {
        return Corpus {
            documents: HashMap::new(),
            terms: HashMap::new()
        }
    }

    fn load_corpus(home_dir: String) -> Corpus {

        let mut corpus: Corpus = Corpus::new();

        let mut corpus_file = fs::File::open(home_dir.clone()+"/.rememberall/corpus.csv").unwrap();
        let mut corpus_buffer = String::new();
        let _ = corpus_file.read_to_string(&mut corpus_buffer);
        let mut corpus_csv_reader = csv::Reader::from_string(corpus_buffer).has_headers(false);
        for row in corpus_csv_reader.decode() {
            let (source, title, text, id): (String, String, String, String) = row.unwrap();
            corpus.documents.insert(id, Document {
                title: title,
                source: source,
                text: text.replace("<br><ul>", "\n    *   ").replace("<ul>", "*   ").replace("<br>","\n       "),
                terms: HashMap::new(),
            });
        }

        // Load the index.
        let mut index_file = fs::File::open(home_dir+"/.rememberall/index.csv").unwrap();
        let mut index_buffer = String::new();
        let _ = index_file.read_to_string(&mut index_buffer);

        let mut index_csv_reader = csv::Reader::from_string(index_buffer).has_headers(false);

        for row in index_csv_reader.decode() {
            let (id, word, tf, tf_idf): (String, String, f32, f32) = row.unwrap();

            match corpus.documents.get_mut(&id) {
                Some(doc) => doc.terms.insert(word.clone(), tf),
                _ => continue
            };
            corpus.terms.insert(word, tf_idf/tf);
        }
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

    fn inverse_document_frequency(&mut self) {
        let number_of_documents = self.documents.len() as f32;

        for (_, document) in &mut self.documents {

            for (term, _) in &mut document.terms {
                // Check if the string is already present in the map. If not, add it.
                let count: f32 = match self.terms.get(term) {
                    Some(count) => count + 1.0f32,
                    _ => 1.0f32,
                };
                self.terms.insert(term.to_string(), count);
            }
        }

        let term_list = self.terms.clone();

        for (term, count) in term_list {
            let idf = number_of_documents/(count + 1.0);
            self.terms.insert(term.to_string(), idf.ln());
        }
    }

    fn save(&self, home_dir: String) {
        let mut corpus_file = fs::File::create(home_dir+"/.rememberall/corpus.csv").unwrap();
        for (id, document) in &self.documents {
            let _ = corpus_file.write_fmt(format_args!("\"{}\",\"{}\",\"{}\",\"{}\"\n",
                                          document.source, document.title, document.text, id));
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
    terms: HashMap<String, f32>,
}

impl Clone for Document {
    fn clone(&self) -> Document {
        return Document {
            title: self.title.clone(),
            source: self.source.clone(),
            text: self.text.clone(),
            terms: self.terms.clone(),
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
                title = line.trim().to_string();
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
                      .to_string(),
            terms: HashMap::new(),
        }

    }

    fn term_frequency(&mut self) {
        // Calculate the term frequency for all of the terms in the document.
        let text = self.text.replace("<ul>"," ").replace("["," ").replace("**"," ").to_string();
        let term_list: Vec<&str> = text.split_whitespace().collect();

        let mut stems: HashSet<String> = HashSet::new();


        for term in term_list {

            // Clense the terms. Make lowercase and remove extra symobls.
            let clean_term = term.to_lowercase().replace(".", "").replace("\t"," ")
            .replace(",","").replace("\"","").replace("<br>", " ").replace("<li>", "");

            let s = stem::get(&clean_term);
            match s {
                Ok(stemmed) => stems.insert(stemmed),
                Err(_) => stems.insert(clean_term.clone()),
            };

        }
        
        let number_of_terms = stems.len() as f32;
       
        for term in stems{

            // Check if the string is already present in the map. If not, add it.
            let count: f32 = match self.terms.get(&term) {
                Some(count) => count + (1.0/ number_of_terms),
                _ => (1.0f32/ number_of_terms),
            };

            self.terms.insert(term.to_string(), count);
        }
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

fn search(args: Args, home_dir: String) {
    let corpus = Corpus::load_corpus(home_dir);
    let mut total_score = 0.0_f32;

    let mut results: Vec<(String, f32)> = Vec::new();
    let mut scaled_results: Vec<(String, u32)> = Vec::new();

    let mut stems: HashSet<String> = HashSet::new();


    for term in &args.arg_term {
            let s = stem::get(&term);
            match s {
                Ok(stemmed) => stems.insert(stemmed),
                Err(_) => stems.insert(term.clone()),
            };
    }

    for (id, document) in &corpus.documents {
        let mut score = 0.0_f32;
        for (cursor_term, tf) in &document.terms {
            let idf: f32 = match corpus.terms.get(cursor_term) {
                Some(value) => *value,
                _ => 0.0_f32
            };
            for term in &args.arg_term {
                let stem_result = stem::get(&term);
                let stem = match stem_result {
                    Ok(stemmed) => stemmed,
                    Err(_) => term.clone(),
                };

                if *cursor_term != *stem {
                    continue;
                }
                score += idf*tf;
            }
        }
        total_score += score;
        if score > 0.0_f32 {
            results.push((id.to_string(), score));
        }
    }

    for (id, score) in results {
        let rank: u32 = ((score/total_score) * 100000.0_f32) as u32;
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
                let score_float: f32 = score as f32;
                let scaled_score = score_float/100000.0_f32;
                let score_string: String = scaled_score.to_string();
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
    corpus.inverse_document_frequency();

    // Calculate the term frequency inverse document frequency and write to disk
    let mut index_file = fs::File::create(home_dir.clone()+"/.rememberall/index.csv").unwrap();
    //let document_list = corpus.documents.clone();
    for (id, document) in &corpus.documents {
        for (term, frequency) in &document.terms {
            // Get the inverse document frequency from the corpus
            let inverse_document_frequency: f32 = match corpus.terms.get(term) {
                Some(count) => *count,
                _ => 0.0_f32,
            };
            let tf_idf = frequency * inverse_document_frequency;
            let _ = index_file.write_fmt(format_args!("\"{}\",\"{}\",\"{}\",{}\n", id, term, frequency, tf_idf));
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

    let mut home_dir: String = String::new();
    
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

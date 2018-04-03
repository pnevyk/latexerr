#![feature(iterator_step_by)]
#![feature(box_syntax)]
#![feature(associated_type_defaults)]

extern crate regex;
extern crate yansi;

mod rules;
mod utils;

use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::env;
use std::fs::File;
use std::io::Read;
use std::iter::FromIterator;

use regex::Regex;
use yansi::Paint;

use rules::{Location, LogItem};

fn get_file_block_indices(log: &str) -> Vec<(usize, usize, String)> {
    let mut open = Vec::new();
    let mut blocks = Vec::new();

    // find start and end indices of all blocks
    for (index, character) in log.chars().enumerate() {
        match character {
            '(' => open.push(index),
            ')' => blocks.push((open.pop().unwrap(), index)),
            _ => {}
        }
    }

    // if there is the emergency stop, the closing parenthesis is missing
    if open.len() > 0 {
        blocks.push((open.pop().unwrap(), log.len()));
    }

    let texfile_pattern = Regex::new(r"^\((.+)\.tex").unwrap();

    // keep only those blocks which represent a tex file and add the filename to the collection
    let mut blocks = blocks
        .into_iter()
        .filter(|(start, end)| texfile_pattern.is_match(&log[*start..*end]))
        .map(|(start, end)| {
            let mat = texfile_pattern
                .captures(&log[start..end])
                .unwrap()
                .get(1)
                .unwrap();
            (start, end, mat.as_str().replace("./", "") + ".tex")
        })
        .collect::<Vec<(usize, usize, String)>>();

    // extend the main file block to the log file end (some errors are behind ending parenthesis)
    let last = blocks.pop().unwrap();
    blocks.push((last.0, log.len(), last.2));

    blocks
}

fn process<'a>(log: &'a str) -> HashMap<String, Vec<LogItem<'a>>> {
    let rules = LogItem::rules();
    let mut output = HashMap::new();

    let blocks = get_file_block_indices(log);

    // find errors and warnings in all blocks
    let mut processed: Vec<(usize, usize)> = Vec::new();
    for (start, end, filename) in blocks {
        let mut parts = Vec::new();
        let mut last = start;

        // get indices of parts which correspond to block's actual content (filter nested blocks)
        for item in &processed {
            if item.0 > last && last >= start && item.0 < end {
                parts.push((last, item.0));
                last = item.1;
            }
        }

        parts.push((last, end));

        // process parts and find errors and warnigs
        let mut log_items = Vec::new();
        for rule in &rules {
            let regex = rule.get_regex();

            for (start, end) in &parts {
                for found in rule.captures(regex.clone(), &log[*start..*end]) {
                    log_items.push(rule.process(found));
                }
            }
        }

        // add this block's indices to those which are already processed
        processed.push((start, end));

        // append rules
        output
            .entry(filename)
            .or_insert(Vec::new())
            .append(&mut log_items);
    }

    // post-process found log items
    output
        .into_iter()
        .map(|(filename, log_items)| {
            // get unique log items
            let mut log_items = Vec::from_iter(
                HashSet::<LogItem<'a>>::from_iter(log_items.into_iter()).into_iter(),
            );

            // sort log items by location
            log_items.sort_unstable_by(|a, b| match (&a.location, &b.location) {
                (Location::Line(a), Location::Line(b)) => a.cmp(&b),
                (Location::Line(_), Location::End) => Ordering::Less,
                (Location::Line(_), Location::None) => Ordering::Greater,
                (Location::End, Location::Line(_)) => Ordering::Greater,
                (Location::None, Location::Line(_)) => Ordering::Less,
                _ => Ordering::Equal,
            });

            (filename, log_items)
        })
        .collect()
}

fn main() {
    let files = env::args()
        .filter(|arg| arg.ends_with(".log"))
        .collect::<Vec<String>>();

    if files.is_empty() {
        eprintln!("No files were passed");
    } else {
        for filename in files {
            match File::open(filename.clone()) {
                Ok(mut file) => {
                    let mut buffer = String::new();
                    file.read_to_string(&mut buffer)
                        .expect(&format!("Cannot read {}", filename));

                    let items = process(&buffer);
                    let count = items.len();

                    // sort files by filename
                    let mut items = items.into_iter().collect::<Vec<(String, Vec<LogItem>)>>();
                    items.sort_unstable_by(|a, b| a.0.cmp(&b.0));

                    // print all source files and corresponding items
                    for (index, (filename, log_items)) in items.iter().enumerate() {
                        println!("{} {}", Paint::cyan("File:"), filename);
                        println!();

                        for log_item in log_items {
                            println!("{}", log_item);
                        }

                        // don't add new line after last file
                        if index < count - 1 {
                            println!();
                        }
                    }
                }
                Err(_) => eprintln!("Cannot read {}", filename),
            }
        }
    }
}

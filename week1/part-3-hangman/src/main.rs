// Simple Hangman Program
// User gets five incorrect guesses
// Word chosen randomly from words.txt
// Inspiration from: https://doc.rust-lang.org/book/ch02-00-guessing-game-tutorial.html
// This assignment will introduce you to some fundamental syntax in Rust:
// - variable declaration
// - string manipulation
// - conditional statements
// - loops
// - vectors
// - files
// - user input
// We've tried to limit/hide Rust's quirks since we'll discuss those details
// more in depth in the coming lectures.
extern crate rand;
use rand::Rng;
use std::fs;
use std::io;
use std::io::Write;

const NUM_INCORRECT_GUESSES: u32 = 5;
const WORDS_PATH: &str = "words.txt";

fn pick_a_random_word() -> String {
    let file_string = fs::read_to_string(WORDS_PATH).expect("Unable to read file.");
    let words: Vec<&str> = file_string.split('\n').collect();
    String::from(words[rand::thread_rng().gen_range(0, words.len())].trim())
}

fn main() {
    let secret_word = pick_a_random_word();
    // Note: given what you know about Rust so far, it's easier to pull characters out of a
    // vector than it is to pull them out of a string. You can get the ith character of
    // secret_word by doing secret_word_chars[i].
    let secret_word_chars: Vec<char> = secret_word.chars().collect();

    // Uncomment for debugging:
    // println!("random word: {}...", secret_word);

    let mut guessed_so_far = String::new();

    let mut guessed = Vec::new();
    for _ in 0..secret_word_chars.len() {
        guessed.push('-');
    }

    let mut incrrect_guesses_count = 0;
    let mut guessed_count = 0;

    loop {
        if incrrect_guesses_count >= NUM_INCORRECT_GUESSES || guessed_count == secret_word.len() {
            break;
        }
        print!("The word so far is ");
        for ele in guessed.iter() {
            print!("{}", ele);
        }
        print!("\n");

        print!(
            "You have guessed the following letters: {}\n",
            guessed_so_far
        );
        print!(
            "You have {} guesses left\n",
            NUM_INCORRECT_GUESSES - incrrect_guesses_count
        );

        print!("Please guess a letter: ");
        io::stdout().flush().expect("Error flushing stdout.");

        let mut guess = String::new();

        io::stdin()
            .read_line(&mut guess)
            .expect("Error reading line.");
        let mut guess_right = false;
        guessed_so_far.push_str(guess.trim());

        for (i, ele) in secret_word_chars.iter().enumerate() {
            // opt: hashset instead of loop
            if guessed[i] == '-' && guess.trim() == ele.to_string() {
                guessed[i] = *ele;
                guess_right = true;
                guessed_count += 1;
                break;
            }
        }

        if !guess_right {
            incrrect_guesses_count += 1;
            print!("Sorry, that letter is not in the word\n");
        }
    }

    if guessed_count == secret_word.len() {
        println!(
            "Congratulations you guessed the secret word: {}!",
            secret_word
        );
    } else {
        println!("Sorry, you ran out of guesses!");
    }
}

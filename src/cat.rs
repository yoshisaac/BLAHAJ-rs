extern crate rand;

use std::io::stdout;
use std::io::Write;

use crate::flags;

// A struct to contain info we need to print with every character
pub struct Control {
    pub seed: usize,
    pub flag_name: String,
    pub background_mode: bool,
    pub individual_mode: bool,
    pub word_mode: bool,
    pub print_color: bool,
    pub terminal_supports_truecolor: bool,
}

// This used to have more of a reason to exist, however now all its functionality is in
// print_chars_lol(). It takes in an iterator over lines and prints them all.
// At the end, it resets the foreground color
pub fn print_lines_lol<I: Iterator<Item = S>, S: AsRef<str>>(lines: I, c: &mut Control) {
    for line in lines {
        print_chars_lol(line.as_ref().chars().chain(Some('\n')), c, false);
    }
    if c.print_color {
        print!("\x1b[39m");
    }
}

// Takes in s an iterator over characters
// duplicates escape sequences, otherwise prints printable characters with colored_print
// Print newlines correctly, resetting background
// If constantly_flush is on, it won't wait till a newline to flush stdout
pub fn print_chars_lol<I: Iterator<Item = char>>(
    mut iter: I,
    c: &mut Control,
    constantly_flush: bool,
) {
    let mut ignoring_whitespace = c.background_mode;
    let mut printed_chars_on_line_plus_one = 1u16;

    if !c.print_color {
        for character in iter {
            print!("{}", character);
        }
        return;
    }

    while let Some(character) = iter.next() {
        match character {
            // Consume escape sequences
            '\x1b' => {
                // Escape sequences seem to be one of many different categories: https://en.wikipedia.org/wiki/ANSI_escape_code
                // CSI sequences are \e \[ [bytes in 0x30-0x3F] [bytes in 0x20-0x2F] [final byte in 0x40-0x7E]
                // nF Escape seq are \e [bytes in 0x20-0x2F] [byte in 0x30-0x7E]
                // Fp Escape seq are \e [byte in 0x30-0x3F] [I have no idea, but `sl` creates one where the next byte is the end of the escape sequence, so assume that]
                // Fe Escape seq are \e [byte in 0x40-0x5F] [I have no idea, '' though sl doesn't make one]
                // Fs Escape seq are \e [byte in 0x60-0x7E] [I have no idea, '' though sl doesn't make one]
                // Otherwise the next byte is the whole escape sequence (maybe? I can't exactly tell, but I will go with it)
                // We will consume up to, but not through, the next printable character
                // In addition, we print everything in the escape sequence, even if it is a color (that will be overriden)
                // TODO figure out just how these should affect printed_characters_on_line
                print!("\x1b");
                let mut escape_sequence_character = iter
                    .next()
                    .expect("Escape character with no escape sequence after it");
                print!("{}", escape_sequence_character);
                match escape_sequence_character {
                    '[' => loop {
                        escape_sequence_character =
                            iter.next().expect("CSI escape sequence did not terminate");
                        print!("{}", escape_sequence_character);
                        match escape_sequence_character {
                            '\x30'..='\x3F' => continue,
                            '\x20'..='\x2F' => {
                                loop {
                                    escape_sequence_character =
                                        iter.next().expect("CSI escape sequence did not terminate");
                                    print!("{}", escape_sequence_character);
                                    match escape_sequence_character {
                            '\x20' ..= '\x2F' => continue,
                            '\x40' ..= '\x7E' => break,
                            _ => panic!("CSI escape sequence terminated with an incorrect value"),
                            }
                                }
                                break;
                            }
                            '\x40'..='\x7E' => break,
                            _ => panic!("CSI escape sequence terminated with an incorrect value"),
                        }
                    },
                    '\x20'..='\x2F' => loop {
                        escape_sequence_character =
                            iter.next().expect("nF escape sequence did not terminate");
                        print!("{}", escape_sequence_character);
                        match escape_sequence_character {
                            '\x20'..='\x2F' => continue,
                            '\x30'..='\x7E' => break,
                            _ => panic!("nF escape sequence terminated with an incorrect value"),
                        }
                    },
                    //            '\x30' ..= '\x3F' => panic!("Fp escape sequences are not supported"),
                    //            '\x40' ..= '\x5F' => panic!("Fe escape sequences are not supported"),
                    //            '\x60' ..= '\x7E' => panic!("Fs escape sequences are not supported"),
                    // be lazy and assume in all other cases we consume exactly 1 byte
                    _ => (),
                }
            }
            // Newlines print escape sequences to end background prints, and in dialup mode sleep, and
            // reset the seed of the coloring and the value of ignore_whitespace	    
	    '\n' => {
                handle_newline(
                    c,
                    &mut ignoring_whitespace,
                    &mut printed_chars_on_line_plus_one,
                );
            }
            // If not an escape sequence or a newline, print a colorful escape sequence and then the
            // character
            _ => {

		if c.background_mode && character.is_whitespace() {
		    print!("\x1b[49m");
		    print!("{}", character);
		} else {
		    colored_print(character, c);
		}
		
		if c.individual_mode && !character.is_whitespace() {
		    c.seed += 1;
		}

		if c.word_mode && character.is_whitespace() {
		    c.seed += 1;
		}
		
		printed_chars_on_line_plus_one += 1;
            }
        }

        // If we should constantly flush, flush after each completed sequence, and also reset
        // colors because otherwise weird things happen
        if constantly_flush {
            reset_colors(c);
            stdout().flush().unwrap();
        }
    }
}


fn handle_newline(
    c: &mut Control,
    ignoring_whitespace: &mut bool,
    printed_chars_on_line_plus_one: &mut u16,
) {
    if c.print_color {
        // Reset the background color only, as we don't have to reset the foreground till
        // the end of the program
        // We reset the background here because otherwise it bleeds all the way to the next line
        if c.background_mode {
            print!("\x1b[49m");
        }
    }
    println!();

    if !c.individual_mode && !c.word_mode {
	c.seed += 1;
    }
    *ignoring_whitespace = c.background_mode;
    *printed_chars_on_line_plus_one = 1u16;
}

fn reset_colors(c: &Control) {
    if c.print_color {
        // Reset the background color
        if c.background_mode {
            print!("\x1b[49m");
        }

        // Reset the foreground color
        print!("\x1b[39m");
    }
}

fn colored_print(character: char, c: &mut Control) {
    if c.background_mode {
        let bg = get_color_tuple(c);
        let fg = calc_fg_color(bg);
        print!(
            "{}{}{}",
            rgb_to_256(fg.0, fg.1, fg.2, true, c.terminal_supports_truecolor),
            rgb_to_256(bg.0, bg.1, bg.2, false, c.terminal_supports_truecolor),
            character
        );
    } else {
        let fg = get_color_tuple(c);
        print!(
            "{}{}",
            rgb_to_256(fg.0, fg.1, fg.2, true, c.terminal_supports_truecolor),
            character
        );
    }
}

fn calc_fg_color(bg: (u8, u8, u8)) -> (u8, u8, u8) {
    // Currently, it only computes the forground clolor based on some threshold
    // on grayscale value.
    // TODO: Add a better algorithm for computing forground color
    if conv_grayscale(bg) > 0xA0_u8 {
        (0u8, 0u8, 0u8)
    } else {
        (0xffu8, 0xffu8, 0xffu8)
    }
}

fn linear_to_srgb(intensity: f64) -> f64 {
    if intensity <= 0.003_130_8 {
        12.92 * intensity
    } else {
        1.055 * intensity.powf(1.0 / 2.4) - 0.055
    }
}

fn srgb_to_linear(intensity: f64) -> f64 {
    if intensity < 0.04045 {
        intensity / 12.92
    } else {
        ((intensity + 0.055) / 1.055).powf(2.4)
    }
}

fn conv_grayscale(color: (u8, u8, u8)) -> u8 {
    // See https://en.wikipedia.org/wiki/Grayscale#Converting_color_to_grayscale
    const SCALE: f64 = 256.0;

    // Changing SRGB to Linear for gamma correction
    let red = srgb_to_linear(f64::from(color.0) / SCALE);
    let green = srgb_to_linear(f64::from(color.1) / SCALE);
    let blue = srgb_to_linear(f64::from(color.2) / SCALE);

    // Converting to grayscale
    let gray_linear = red * 0.299 + green * 0.587 + blue * 0.114;

    // Gamma correction
    let gray_srgb = linear_to_srgb(gray_linear);

    (gray_srgb * SCALE) as u8
}

fn get_color_tuple(c: &Control) -> (u8, u8, u8) {
    let flag_color = flags::get_flag(&c.flag_name);
    flag_color[c.seed % flag_color.len()]
}

// Returns closest supported 256-color an RGB value
// Inspired by the ruby paint gem
fn rgb_to_256(r: u8, g: u8, b: u8, foreground: bool, use_truecolor: bool) -> String {
    let prefix = if foreground { "38" } else { "48" };

    if use_truecolor {
        return format!("\x1b[{};2;{};{};{}m", prefix, r, g, b);
    }

    let r = r as f64;
    let g = g as f64;
    let b = b as f64;

    let colors = [(r, 36), (g, 6), (b, 1)];
    let mut color_code = 16;
    for (color, modulator) in &colors {
        color_code += ((6.0 * (*color / 256.0)).floor() as u16) * modulator;
    }

    format!("\x1b[{};5;{}m", prefix, color_code)
}

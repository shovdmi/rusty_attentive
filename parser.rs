#![deny(unsafe_code)]
#![allow(dead_code)]
#![allow(non_camel_case_types)]
//TODO: fix this
#![allow(non_upper_case_globals)]

const PARSER_BUF_SIZE: usize = 128;

const final_ok_responses: &[&[u8]] = &[b"OK"];

const final_responses: &[&[u8]] = &[
    b"OK",
    b"ERROR",
    b"NO CARRIER",
    b"+CME ERROR:",
    b"+CMS ERROR:",
];

const urc_responses: &[&[u8]] = &[b"RING"];

#[derive(Debug)]
enum at_parser_state {
    IDLE,
    READLINE,
    DATAPROMPT,
    RAWDATA,
    HEXDATA,
}

#[derive(Debug)]
enum at_response_type {
    UNEXPECTED,                      // Unexpected line; usually an unhandled URC.
    UNKNOWN,                         // Pass the response to next parser in chain.
    INTERMEDIATE,                    // Intermediate response. Stored.
    FINAL_OK,                        // Final response. NOT stored.
    FINAL,                           // Final response. Stored.
    URC,                             // Unsolicited Result Code. Passed to URC handler.
    RAWDATA_FOLLOWS { amount: usize }, // rust's enum feature
    HEXDATA_FOLLOWS { amount: usize }, // rust's enum feature
}

//https://stackoverflow.com/questions/41081240/idiomatic-callbacks-in-rust/41081702
//https://users.rust-lang.org/t/solved-how-to-pass-none-to-a-function-when-an-option-closure-is-expected/10956/2
struct callback_func {
    handler_func: Option<fn(&str)>,
}
impl callback_func {
    fn try_to_call(self, s: &str) {
        match self.handler_func {
            Some(f) => f(s),
            None => println!("No user-handler defined"),
        }
    }
}

struct callbacks {
    scan_line: Option<fn(&[u8]) -> at_response_type>,
    handle_response: Option<fn(&[u8])>,
    handle_urc: Option<fn(&[u8])>,
}
impl callbacks {}

struct Parser {
    state: at_parser_state,
    expect_data_promt: bool,
    data_left: usize,
    nibble: i8,

    buf: [u8; 128],
    buf_used: usize,
    buf_size: usize,
    buf_current: usize,

    cbs: callbacks,
}

impl Parser {
    ///
    ///
    ///
    fn reset(&mut self) {
        println!("\tresetting parser");

        self.expect_data_promt = false;
        self.data_left = 0;
        self.nibble = 0;

        self.buf = [b'\0'; 128];
        self.buf_used = 0;
        //self.buf_size = PARSER_BUF_SIZE;
        self.buf_current = 0;
        self.state = at_parser_state::IDLE;
    }

    ///
    ///
    ///
    fn append(&mut self, ch: u8) {
        println!("\tparser append '0x{:X?}' as char '{}'", ch, ch as char);
        if self.buf_used < self.buf_size - 1 {
            self.buf[self.buf_used as usize] = ch;
            self.buf_used += 1;
        }
    }

    ///
    ///
    ///
    fn at_prefix_in_table(&self, line: &[u8], table: &[&[u8]]) -> bool {
        for i in 0..table.len() {
            if line == table[i] {
                use std::str;
                if let Ok(s) = str::from_utf8(table[i]) {
                    println!("FOUND: \"{}\"", s);
                    return true;
                };
                println!("\t{:X?} at {} pos", table[i], i);
            }
        }
        false
    }
    ///
    ///
    ///
    fn discard_line(&mut self) {
        println!("parser discarding line");
        //Rewind the end pointer back to the previous position.
        self.buf_used = self.buf_current;
    }
    ///
    ///
    ///
    fn include_line(&mut self) {
        println!("parser including line");
        self.append(b'\n');
        self.buf_current = self.buf_used;
    }
    ///
    ///
    ///
    fn handle_urc(&mut self) {
        let line = &self.buf[self.buf_current..self.buf_used];
        match self.cbs.handle_urc {
                Some(f) => f(&line),
                None => {
                    println!("\t\tNo 'URC' user-handler defined");
                    // do nothing
                }
        };
        self.discard_line();
    }
    
    ///
    ///
    ///
    fn finalize(&self){
    }
    ///
    ///
    ///
    fn handle_final_response(&mut self) {
        println!("parser final response");
        self.finalize();
        let line = &self.buf[0..self.buf_used];
        match self.cbs.handle_response {
                Some(f) => f(&line),
                None => {
                    println!("\t\tNo 'response' user-handler defined");
                    // do nothing
                }
        };
        self.reset();
    }
    ///
    ///
    ///
    fn generic_scan_line(&self, line: &[u8]) -> at_response_type {
        println!("generic(parser's) scan line : {:X?}", line);
        use at_response_type::*;
        match self.state {
            at_parser_state::DATAPROMPT => {
                if line.len() == 2 && line == b"> " {
                    return FINAL_OK;
                }
            }
            _ => {}
        }

        if self.at_prefix_in_table(&line, &urc_responses) {
            return URC;
        } else if self.at_prefix_in_table(&line, &final_ok_responses) {
            return FINAL_OK;
        } else if self.at_prefix_in_table(&line, &final_responses) {
            return FINAL;
        }

        INTERMEDIATE
    }
    ///
    ///
    ///
    fn handle_line(&mut self) {
        println!("\tparser handle line");

        //Skip empty lines
        if self.buf_used == self.buf_current {
            return;
        }

        //TODO: NULL-terminate the response .
        //parser->buf[parser->buf_used] = '\0';
        
        //Extract line address & length for later use.
        let line = &self.buf[self.buf_current as usize..self.buf_used as usize];
        println!("\t\tline: {:X?} len:{}", line, line.len());

        {
            use std::str;
            let sl: &str = match str::from_utf8(&line) {
                Ok(v) => {
                    println!("\t\tstr slice: {:?} len:{}", v, v.len());
                    v
                }
                Err(e) => panic!("Invalid UTF-8 sequence: {}", e),
            };
        }

        //Determine response type.
        let response_type = match self.cbs.scan_line {
            Some(f) => f(&line),
            None => {
                println!("\t\tNo user-handler defined");
                self.generic_scan_line(line)
            }
        };

        println!("Response type: {:#?}", response_type);
        // Expected URCs and all unexpected lines are sent to URC handler.
        // parser->state == STATE_IDLE -- means, we are in the idle state,
        // and suddenly received a string+\n (such as "RING\n")
        // so we treat that as an incoming URC
        // type == AT_RESPONSE_URC means we are awaiting a response
        // to the AT command, and during threre was a string+\n (such as "RING\n")
        //if (type == AT_RESPONSE_URC || parser->state == STATE_IDLE)
        match (&self.state, &response_type) {
            // https://doc.rust-lang.org/book/ch18-03-pattern-syntax.html#ignoring-parts-of-a-value-with-a-nested-_
            (at_parser_state::IDLE, _ ) | (_, at_response_type::URC) => {
                drop(line);
                self.handle_urc();
                return;
            },
            _ => {},
        };
        
        match (&response_type) {
            at_response_type::FINAL_OK => self.discard_line(),
            _ => self.include_line(),
        };
        
        use at_response_type::*;
        match (response_type) {
            FINAL | FINAL_OK => self.handle_final_response(),
            RAWDATA_FOLLOWS{amount} => {self.data_left = amount;
                                        self.state = at_parser_state::RAWDATA;
                                },
            HEXDATA_FOLLOWS{amount} => {self.data_left = amount; 
                                        self.nibble = -1; 
                                        self.state = at_parser_state::HEXDATA;
                                },
            _ => {},
        };
    }

    ///
    ///
    ///
    fn feed(&mut self, st: &[u8]) {
        println!("\tparser state {:?} feed (\"{:?}\")", self.state, st);

        for ch in st {
            println!("{}", ch);

            use at_parser_state::*;
            match self.state {
                IDLE | READLINE => {
                    if ch != &b'\r' && ch != &b'\n' {
                        self.append(*ch);
                    }
                    if ch == &b'\n' {
                        self.handle_line();
                    }
                }
                
                DATAPROMPT => {
                    if self.buf_used == 2 && self.buf[0] == b'>' && self.buf[1] == b' ' {
                        self.handle_line();
                    }
                }

                RAWDATA => {}
                HEXDATA => {}
            }
        }
    }
}

fn user_scan_line(s: &[u8]) {
    println!("user callback scanline: {:?}", s);
}

fn main() {
    // let scan_line = |&s| println!("generic callback scanline: {}", &s);
    // let handle_response = |&s|  println!("generic callback handle_response: {}", &s);
    // let handle_urc = |&s| println!("generic callback handle_urc {}", &s);

    let mut parser = Parser {
        state: at_parser_state::IDLE,
        expect_data_promt: false,
        data_left: 0,
        nibble: 0,

        buf: [b'\0'; 128],
        buf_used: 0,
        buf_size: PARSER_BUF_SIZE,
        buf_current: 0,

        cbs: callbacks {
            scan_line: None, //Some(user_scan_line),
            handle_response: None,
            handle_urc: None,
        },
    };

    parser.reset();

    let response = b"\rRING\r\n";
    parser.feed(response);
    
    let response = b"\rOK\r\n";
    parser.feed(response);

    parser.state = at_parser_state::READLINE;
    let response = b"+CME ERROR:\r\nOK\r\n";
    parser.feed(response);

    parser.state = at_parser_state::DATAPROMPT;
    let response = b"> go\r\n";
    parser.feed(response);
}

#![deny(unsafe_code)]
#![allow(dead_code)]
#![allow(non_camel_case_types)]
#![no_mangle]
//TODO: fix this
#![allow(non_upper_case_globals)]

/* macro_rules! println {
    () => (print!("\n"));
    ($($arg:tt)*) => ({
        //$crate::io::_print(format_args_nl!($($arg)*));
    })
}*/

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
    UNEXPECTED,                        // Unexpected line; usually an unhandled URC.
    UNKNOWN,                           // Pass the response to next parser in chain.
    INTERMEDIATE,                      // Intermediate response. Stored.
    FINAL_OK,                          // Final response. NOT stored.
    FINAL,                             // Final response. Stored.
    URC,                               // Unsolicited Result Code. Passed to URC handler.
    RAWDATA_FOLLOWS { amount: usize }, // rust's enum feature
    HEXDATA_FOLLOWS { amount: usize }, // rust's enum feature
}

/*
//https://stackoverflow.com/questions/41081240/idiomatic-callbacks-in-rust/41081702
//https://users.rust-lang.org/t/solved-how-to-pass-none-to-a-function-when-an-option-closure-is-expected/10956/2
struct callback_func {
    handler_func: Option<fn(&str)>,
}
impl callback_func {
    fn try_to_call(self, s: &str) {
        if let Some(f) = self.handler_func {
            f(s)
        } else {
            println!("No user-handler defined")
        };
    }
}*/
//------------------------------------------------------------------------------
fn print_array_as_str(s: &str, line: &[u8], e: &str) {
    use std::str;
    match str::from_utf8(&line) {
        Ok(v) => {
            print!("{}{:?} len:{} {}", s, v, v.len(), e);
            v
        }
        Err(e) => panic!("Invalid UTF-8 sequence: {}", e),
    };
}
//------------------------------------------------------------------------------

fn hex2int(c: u8) -> i16 {
    if c >= b'0' && c <= b'9' {
        return (c - b'0') as i16;
    }
    if c >= b'A' && c <= b'F' {
        return (c - b'A' + 10) as i16;
    }
    if c >= b'a' && c <= b'f' {
        return (c - b'a' + 10) as i16;
    }

    return -1;
}


struct callbacks {
    scan_line: Option<fn(&[u8]) -> at_response_type>,
    handle_response: Option<fn(&[u8])>,
    handle_urc: Option<fn(&[u8])>,
}
impl callbacks {}



#[repr(C, align(8))]
struct Parser {
    buf: [u8; PARSER_BUF_SIZE],

    state: at_parser_state,
    expect_dataprompt: bool,
    data_left: usize,
    nibble: i16,

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

        self.expect_dataprompt = false;
        self.data_left = 0;
        self.nibble = 0;
        //self.buf = [b'\0'; PARSER_BUF_SIZE];
        self.buf_used = 0;
        //self.buf_size = PARSER_BUF_SIZE;
        self.buf_current = 0;
        self.state = at_parser_state::IDLE;
    }
    
    ///
    ///
    ///
    fn await_response(&mut self) {
        self.state = if self.expect_dataprompt { at_parser_state::DATAPROMPT }
                     else { at_parser_state::READLINE };
        println!("await_respone(): new parser state [{:?}]", self.state);
    }
    
    ///
    ///
    ///
    fn append(&mut self, ch: u8) {
        println!("\tparser append {:?} 0x{:02X?}", ch as char, ch);
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
                print_array_as_str("FOUND:\t", &table[i], "");
                println!("\t{:X?} at {} pos", table[i], i);
                return true;
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
        /*match self.cbs.handle_urc {
            Some(f) => f(&line),
            None => {
                println!("\t\tNo 'URC' user-handler defined");
                // do nothing
            }
        }; */
        if let Some(f) = self.cbs.handle_urc {
            f(&line)
        } else {
            println!("\t\tNo 'URC' user-handler defined")
        }

        self.discard_line();
    }

    ///
    ///
    ///
    fn finalize(&mut self) {
        /* Remove the last newline, if any. */
		if self.buf_used > 0 {
			self.buf_used-=1;
		}

		/* NULL-terminate the response. */
		self.buf[self.buf_used] = b'\0';
    }

    ///
    ///
    ///
    fn handle_final_response(&mut self) {
        print!("parser final response: ");
        self.finalize();
        let line = &self.buf[0..self.buf_used];

        print_array_as_str(&" handling final response:", &line, "\n");

        if let Some(f) = self.cbs.handle_response {
            f(&line)
        } else {
            println!("\t\tNo 'response' user-handler defined");
        };

        self.reset();
    }

    ///
    ///
    ///
    fn generic_scan_line(&self, line: &[u8]) -> at_response_type {
        print!("generic(parser's) scan line :");
        print_array_as_str("\t", &line, "\n");
        println!("\t{:02X?}", line);

        use at_response_type::*;

        if let at_parser_state::DATAPROMPT = self.state {
            if line.len() == 2 && line == b"> " {
                return FINAL_OK;
            }
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
        print!("\tparser handle line ");

        //Skip empty lines
        if self.buf_used == self.buf_current {
            println!("\nSkip empty line");
            return;
        }

        //Extract line address & length for later use.
        let line = &self.buf[self.buf_current..self.buf_used];
        print_array_as_str(&"", &line, "\n");
        println!("\t{:02X?} len:{}", line, line.len());

        //Determine response type.
        use at_response_type::*;

        let mut response_type = UNKNOWN;

        if let Some(f) = self.cbs.scan_line {
            response_type = f(&line);
            println!(
                "Response type(from 'scan_line' callback): {:#?}",
                response_type
            );
        } else {
            println!("\t\tNo user-handler defined");
        }

        if let UNKNOWN = response_type {
            response_type = self.generic_scan_line(line);
            println!(
                "Response type(from generic 'scan_line'): {:#?}",
                response_type
            );
        }

        // Expected URCs and all unexpected lines are sent to URC handler.
        // parser->state == STATE_IDLE -- means, we are in the idle state,
        // and suddenly received a string+\n (such as "RING\n")
        // so we treat that as an incoming URC
        // type == AT_RESPONSE_URC means we are awaiting a response
        // to the AT command, and during threre was a string+\n (such as "RING\n")
        //if (type == AT_RESPONSE_URC || parser->state == STATE_IDLE)
        match (&self.state, &response_type) {
            (at_parser_state::IDLE, _) => {
                drop(line);
                println!("at IDLE any massages are treated as URC");
                self.handle_urc();
                return;
            }
            (_, at_response_type::URC) => {
                drop(line);
                println!("incoming /URC/ during READLINE, DATAPROMT, HEXDATA or RAWDATA");
                self.handle_urc();
                return;
            }
            _ => {}
        };

        match &response_type {
            FINAL_OK => self.discard_line(),
            _ => self.include_line(),
        };

        match response_type {
            FINAL | FINAL_OK => self.handle_final_response(),
            RAWDATA_FOLLOWS { amount } => {
                self.data_left = amount;
                self.state = at_parser_state::RAWDATA;
            }
            HEXDATA_FOLLOWS { amount } => {
                self.data_left = amount;
                self.nibble = -1;
                self.state = at_parser_state::HEXDATA;
            }
            _ => {}
        };
    }

    ///
    ///
    ///
    fn feed(&mut self, st: &[u8]) {
        print!("\tparser state {:?} feed ", self.state);
        print_array_as_str("", &st, "\n");
        println!("\t(\"{:02X?}\")", st);

        for ch in st {
            print!("[{:?}] {:?} 0x{:02X} ", self.state, *ch as char, ch);

            use at_parser_state::*;
            match self.state {
                IDLE | READLINE => {
                    if ch != &b'\r' && ch != &b'\n' {
                        self.append(*ch);
                    } else {
                        println!("\tskipping \\r or \\n");
                    }
                    if ch == &b'\n' {
                        self.handle_line();
                    }
                }

                DATAPROMPT => {
                    if ch != &b'\r' && ch != &b'\n' {
                        self.append(*ch);
                    }
                    //if self.buf_used == 2 && self.buf[0] == b'>' && self.buf[1] == b' ' {
                    if self.buf_used == 2 && &self.buf[0..2] == b"> " {
                        println!("dataprompt captured");
                        self.handle_line();
                    }
                }

                RAWDATA => {
                    if self.data_left > 0 {
                        self.append(*ch);
                        self.data_left -= 1;
                        println!("\tdata left {}", self.data_left);
                    }
                    if self.data_left == 0 {
                        self.include_line();
                        self.state = READLINE;
                    }
                }

                HEXDATA => {
                    if self.data_left > 0 {
                        let mut value = hex2int(*ch);
                        if value != -1 {
                            if self.nibble == -1 {
                                self.nibble = value;
                            } else {
                                value |= self.nibble << 4;
                                self.nibble = -1;
                                self.append(value as u8);
                                self.data_left -= 1;
                                println!("\tdata left {}", self.data_left);
                            }
                        }
                    }
                    if self.data_left == 0 {
                        self.include_line();
                        self.state = READLINE;
                    }
                }
            }
        }
    }
}

fn user_scan_line(s: &[u8]) -> at_response_type {
    print!("user callback 'scan_line': ");
    print_array_as_str("", &s, "\n");
    println!("\t{:02X?}", s);
    at_response_type::UNKNOWN
}

fn user_handle_response(s: &[u8]) {
    print!("user callback 'handle response': ");
    print_array_as_str("", &s, "\n");
    println!("\t{:02X?}", s);
}

fn user_handle_urc(s: &[u8]) {
    print!("user callback 'handle urc': ");
    print_array_as_str("", &s, "\n");
    println!("\t{:02X?}", s);
}



fn main() {
    let mut parser = Parser {
        state: at_parser_state::IDLE,
        expect_dataprompt: false,
        data_left: 0,
        nibble: 0,

        buf: [b'\0'; PARSER_BUF_SIZE],
        buf_used: 0,
        buf_size: PARSER_BUF_SIZE,
        buf_current: 0,

        cbs: callbacks {
            scan_line: Some(user_scan_line),
            handle_response: Some(user_handle_response), //None,
            handle_urc: None,                            //Some(user_handle_urc),           //None,
        },
    };

    parser.reset();
    println!("\n1. --------------------------");
    let response = b"\rRING\r\n";
    parser.feed(response);

    println!("\n2. --------------------------");
    let response = b"\rOK\r\n";
    parser.feed(response);

    println!("\n3. --------------------------");
    parser.state = at_parser_state::READLINE;
    let response = b"OK\r\n";
    parser.feed(response);

    println!("\n4. --------------------------");
    parser.state = at_parser_state::READLINE;
    let response = b"+CME ERROR:\r\nOK\r\n";
    parser.feed(response);

    println!("\n5. --------------------------");
    parser.state = at_parser_state::DATAPROMPT;
    let response = b"> go\r\n";
    parser.feed(response);
    println!("- - - - - - - - - - - - - -");
    parser.state = at_parser_state::READLINE;
    let response = b"\rOK\r\n";
    parser.feed(response);

    println!("\n6. --------------------------");
    parser.state = at_parser_state::READLINE;
    let response = b"intermediate\r\n";
    parser.feed(response);
    println!("- - - - - - - - - - - - - -");
    let response = b"\rOK\r\n";
    parser.feed(response);

    println!("\n7. --------------------------");
    parser.state = at_parser_state::RAWDATA;
    parser.data_left = 10;
    let response = b"RAW\r12\n345";
    parser.feed(response);
    let response = b"OK\n";
    parser.feed(response);

    println!("\n8. --------------------------");
    parser.state = at_parser_state::READLINE;
    let response = b"RING\r\nOK\r\n";
    parser.feed(response);
}

#[cfg(test)]
mod tests {
    fn tester(response: &[u8], expected_response: &[u8], test_name: &str) {
        if response == expected_response {
            println!("TEST PASSED [{}]:", test_name);
        }
        else {
            println!("TEST FAILED [{}]:", test_name);
        }
        print_array_as_str("\tresponse: ", &response, "\n");
        print_array_as_str("\texpected: ", &expected_response, "\n");
        assert_eq!(response, expected_response);
    }
    
    use super::*;
    
    #[test]
    fn test_parser_response() {
        let mut parser = Parser {
            state: at_parser_state::IDLE,
            expect_dataprompt: false,
            data_left: 0,
            nibble: 0,
    
            buf: [b'\0'; PARSER_BUF_SIZE],
            buf_used: 0,
            buf_size: PARSER_BUF_SIZE,
            buf_current: 0,
    
            cbs: callbacks {
                scan_line: None,
                handle_response: None, //None,
                handle_urc: None,                            //Some(user_handle_urc),           //None,
            },
        };

        parser.reset();
        

        let tester_fn = |x:&[u8]| tester(&x, b"ERROR", "Test1");
        parser.cbs.handle_response = Some(tester_fn);
        parser.await_response();
        parser.feed(b"\r\nERROR\r\n");
        
        let tester_fn = |x:&[u8]| tester(&x, b"", "Test2.1");
        parser.cbs.handle_response = Some(tester_fn);
        parser.await_response();
        parser.feed(b"\r\n\r\n\r\n\r\n\r\n");
        let tester_fn = |x:&[u8]| tester(&x, b"ERROR", "Test2.2");
        parser.cbs.handle_response = Some(tester_fn);
        //parser.await_response();
        parser.feed(b"ERROR\r\n");
        
        let tester_fn = |x:&[u8]| tester(&x, b"", "Test3");
        parser.cbs.handle_response = Some(tester_fn);
        parser.await_response();
        parser.feed(b"OK\r\n");
        
        let tester_fn = |x:&[u8]| tester(&x, b"123456789", "Test4");
        parser.cbs.handle_response = Some(tester_fn);
        parser.await_response();
        parser.feed(b"123456789\r\nOK\r\n");
       
        let tester_fn = |x:&[u8]| tester(&x, b"123456789\nERROR", "Test5");
        parser.cbs.handle_response = Some(tester_fn);
        parser.await_response();
        parser.feed(b"123456789\r\nERROR\r\n");
    }
}

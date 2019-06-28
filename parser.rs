#[allow(dead_code)]
#[allow(non_camel_case_types)]


enum at_parser_state {
    IDLE,
    READLINE,
    DATAPROMPT,
    RAWDATA,
    HEXDATA,
}

enum at_response_type {
    UNEXPECTED,         // Unexpected line; usually an unhandled URC.
    UNKNOWN,            // Pass the response to next parser in chain.
    INTERMEDIATE,       // Intermediate response. Stored.
    FINAL_OK,           // Final response. NOT stored.
    FINAL,              // Final response. Stored.
    URC,                // Unsolicited Result Code. Passed to URC handler.
    RAWDATA_FOLLOWS {amount: i32},  // rust's enum feature
    HEXDATA_FOLLOWS {amount: i32},  // rust's enum feature
}

struct Parser<cb_scan_line, cb_handle_response, cb_handle_urc> 
where cb_scan_line: FnMut(), cb_handle_response: FnMut() , cb_handle_urc: FnMut() {
    state: at_parser_state,
    expect_data_promt: bool,
    data_left: u32,
    nibbles: u32,

    buf: [char; 128],
    buf_used: u32,
    buf_size: u32,
    buf_current: u32,
    
    scan_line : cb_scan_line,
    handle_response : cb_handle_response,
    handle_urc : cb_handle_urc,
}

impl<cb_scan_line, cb_handle_response, cb_handle_urc> 
    Parser<cb_scan_line, cb_handle_response, cb_handle_urc> 
    where cb_scan_line: FnMut(), cb_handle_response: FnMut(), cb_handle_urc: FnMut() {
    ///
    ///
    ///
    fn reset(&mut self) {
        println!("\tresetting parser");

        self.expect_data_promt = false;
        self.data_left = 0;
        self.nibbles = 0;

        self.buf = ['\0'; 128];
        self.buf_used = 0;
        self.buf_size = 0;
        self.buf_current = 0;
        self.state = at_parser_state::IDLE;
    }

    ///
    ///
    ///
    fn append(&mut self, ch: char) {
        println!("\tparser append {}", ch);
        if self.buf_used < self.buf_size - 1 {
            self.buf[self.buf_used as usize] = ch;
            self.buf_used += 1;
        }
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
        let line = &self.buf[self.buf_current as usize .. self.buf_used as usize];
        let len = self.buf_used - self.buf_current;
        println!("\tline: {:?} len:{}", line, len);
        
        //Determine response type.
        let mut response_type = at_response_type::UNKNOWN;
        
        //https://stackoverflow.com/questions/41081240/idiomatic-callbacks-in-rust/41081702
        //https://users.rust-lang.org/t/solved-how-to-pass-none-to-a-function-when-an-option-closure-is-expected/10956/2
   /*     if self.cbs.scanline != None
            response_type = self.cbs.scanline();
        
        if response_type == at_response_type::UNKNOWN
            response_type = self.generic_line_scanner(&line, len);
     */   
        
    }
    
    ///
    ///
    ///
    fn feed(&mut self, st: &str) {
    
        println!("\tparser feed \"{}\"", st);
        
        let chars: Vec<char> = st.chars().collect();

        for ch in chars {
            println!("{}", ch);
    
            match self.state {
                at_parser_state::IDLE => {
                    println!("[idle]");
                    if ch != '\r' && ch != '\n' {
                        self.append(ch);
                    }
                    if ch == '\n' {
                        self.handle_line();
                    }
                }
    
                at_parser_state::READLINE => println!("[read line]"),
                
                at_parser_state::DATAPROMPT => {
                    println!("[data promt]");
                    if self.buf_used == 2 && self.buf[0] == '>' && self.buf[1] == ' ' {
                        self.handle_line();
                    }
                },
                    
                at_parser_state::RAWDATA => println!("[raw data]"),
                at_parser_state::HEXDATA => println!("[hex data]"),
            }
        }

    }
}

fn main() {
    let scan_line = || println!("generic callback scanline");
    let handle_response = ||  println!("generic callback handle_response");
    let handle_urc = || println!("generic callback handle_urc");
    
    let mut parser = Parser {
    
        state: at_parser_state::IDLE,
        expect_data_promt: false,
        data_left: 0,
        nibbles: 0,

        buf: ['\0'; 128],
        buf_used: 0,
        buf_size: 0,
        buf_current: 0,
        
        scan_line : scan_line,
        handle_response :  handle_response,
        handle_urc : handle_urc,
    };

    parser.reset();

    let response = "OK\r\n";
    parser.feed(response);
    
    parser.state = at_parser_state::DATAPROMPT;
    let response = "> go\r\n";
    parser.feed(response);
    
}

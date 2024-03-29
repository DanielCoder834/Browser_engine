use std::collections::HashMap;
use crate::css_parser::{Rule, Selector};
use crate::dom;

struct Parser {
    pos: usize,
    input: String,
}

impl Parser {
    // Read the current character without consuming it
    fn next_char(&self) -> char {
        self.input[self.pos..].chars().next().unwrap()
    }

    // Does the next character start with the given string?
    fn starts_with(&self, s: &str) -> bool {
        self.input[self.pos..].starts_with(s)
    }

    // Returns true if all inputs are consumed
    fn eof(&self) -> bool {
        self.pos >= self.input.len()
    }

    // Return the current character, and advance self.pos to the next character
    fn consume_char(&mut self) -> char {
        let mut iter = self.input[self.pos..].char_indices();
        let (_, cur_char) = iter.next().unwrap();
        let (next_pos, _) = iter.next().unwrap_or((1, ' '));
        self.pos += next_pos;
        return cur_char;
    }

    // Consumes characters 'test' returns false
    fn consume_while<F>(&mut self, test: F) -> String
        where F: Fn(char) -> bool {
        let mut result = String::new();
        while !self.eof() && test(self.next_char()) {
            result.push(self.consume_char())
        }
        return result;
    }

    // Consume and discard zero or more whitespace characters
    fn consume_whitespace(&mut self) {
        // Supposed to be "self.consume_while(CharExt::is_whitespace);"
        self.consume_while(self.next_char().is_whitespace());
    }

    // Parse a tag or attribute name
    fn parse_tag_name(&mut self) -> String {
        self.consume_while(|c| match c {
            'a'...'z' | 'A'...'Z' | '0'...'9' => true,
            _ => false
        })
    }

    // Parse a single node
    fn parse_node(&mut self) -> dom::Node {
        match self.next_char() {
            '<' => self.parse_element(),
            _ => self.parse_text()
        }
    }

    // Parse a text node
    fn parse_text(&mut self) -> dom::Node {
        dom::text(self.consume_while(|c| c != '<'))
    }

    // Parse a single element, including its open tag, contents, and closing tags
    fn parse_elements(&mut self) -> dom::Node {
        // Opening tag,
        assert_eq!(self.consume_char(), '<');
        let tag_name = self.parse_tag_name();
        let attrs = self.parse_attributes();
        assert_eq!(self.consume_char(), '>');

        // Contents
        let children = self.parse_nodes();

        // Closing tags
        assert_eq!(self.consume_char(), '<');
        assert_eq!(self.consume_char(), '/');
        assert_eq!(self.parse_tag_name(), tag_name);
        assert_eq!(self.consume_char(), '>');

        return dom::elem(tag_name, attrs, children)
    }

    // Parse a single name="value" pair.
    fn parse_attr(&mut self) -> (String, String) {
        let name = self.parse_tag_name();
        assert_eq!(self.consume_char(), '=');
        let value = self.parse_attr_value();
        return (name, value);
    }

    // Parse a quoted value
    fn parse_attr_value(&mut self) -> String {
        let open_quote = self.consume_char();
        assert!(open_quote == '"' || open_quote == '\'');
        let value = self.consume_while(|c| c != open_quote);
        assert_eq!(self.consume_char(), open_quote);
        return value
    }

    // Parse a list of name="value" pairs, separated by whitespace.
    fn parse_attributes(&mut self) -> dom::AttrMap {
        let mut attributes = HashMap::new();
        loop {
            self.consume_whitespace();
            if self.next_char() == '>' {
                break;
            }
            let (name, value) = self.parse_attr();
            attributes.insert(name, value);
        }
        return attributes;
    }

    // Parse a sequence of sibling nodes.
    fn parse_nodes(&mut self) -> Vec<dom::Node> {
        let mut nodes = Vec::new();
        loop {
            self.consume_whitespace();
            if self.eof() || self.starts_with("</") {
                break;
            }
            nodes.push(self.parse_node());
        }
        return nodes;
    }

    // Parse an HTML document and return the root element.
    pub fn parse(source: String) -> dom::Node {
        let mut nodes = Parser { pos: 0, input: source }.parse_nodes();

        // If the document contains a root element, just return it. Otherwise, create one.
        if nodes.len() == 1 {
            nodes.swap_remove(0)
        } else {
            dom::elem("html".to_string(), HashMap::new(), nodes)
        }
    }

    fn parse_simple_selector(&mut self) -> crate::css_parser::SimpleSelector {
        let mut selector = crate::css_parser::SimpleSelector { tag_name: None, id: None, class: Vec::new() };
        while !self.eof() {
            match self.next_char() {
                '#' => {
                    self.consume_char();
                    selector.id = Some(self.parse_identifier());
                }
                '.' => {
                    self.consume_char();
                    selector.class.push(self.parse_identifier());
                }
                '*' => {
                    // Universal selector
                    self.consume_char();
                }
                c if valid_identifier_char(c) => {
                    selector.tag_name = Some(self.parse_identifier());
                }
                _ => {
                    break
                }
            }
        }
        return selector
    }



    // Parse a rule set: `<selector> { <declarations> }`.
    fn parse_rule(&mut self) -> Rule {
        Rule {
            selectors: self.parse_selectors(),
            declarations: self.parse_declarations()
        }
    }

    // Parse a comma-separated list of selectors.
    fn parse_selectors(&mut self) -> Vec<Selector> {
        let mut selectors = Vec::new();
        loop {
            selectors.push(Selector::Simple(self.parse_simple_selector()));
            self.consume_whitespace();
            match self.next_char() {
                ',' => { self.consume_char(); self.consume_whitespace(); }
                '{' => break, //Start of declarations
                c => panic!("Unexpected character {} in selector list", c)
            }
        }
        // Return selectors with highest specificity first, for use in matching
        selectors.sort_by(|a, b| b.specificity().cmp(&a.specificity()));
        return selectors;
    }
}

fn valid_identifier_char(c: char) -> bool {
    match c {
        'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' => true, // TODO: Include U+00A0 and higher.
        _ => false,
    }
}